use muster_lib::moodle::throttle::{RequestGate, ThrottleConfig};
use muster_lib::moodle::auth::MoodleAuth;
use muster_lib::moodle::scraper::MoodleScraper;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_request_gate_burst_pacing_and_concurrency_cap() {
    let min_interval = Duration::from_millis(25);
    let max_concurrency = 4;
    let num_requests = 20;

    let gate = Arc::new(RequestGate::new(ThrottleConfig {
        min_interval,
        max_concurrency,
    }));

    let active_count = Arc::new(AtomicUsize::new(0));
    let max_observed_concurrency = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    let start_all = Instant::now();

    for i in 0..num_requests {
        let gate_clone = gate.clone();
        let active_clone = active_count.clone();
        let max_obs_clone = max_observed_concurrency.clone();

        handles.push(tokio::spawn(async move {
            let permit = gate_clone.acquire().await;
            let dispatch_time = Instant::now();

            let cur = active_clone.fetch_add(1, Ordering::SeqCst) + 1;
            max_obs_clone.fetch_max(cur, Ordering::SeqCst);

            // Simulate simulated network request duration
            tokio::time::sleep(Duration::from_millis(15)).await;

            active_clone.fetch_sub(1, Ordering::SeqCst);
            drop(permit);

            (i, dispatch_time)
        }));
    }

    let mut dispatch_times = Vec::new();
    for h in handles {
        let (id, t) = h.await.expect("task must not panic");
        dispatch_times.push((id, t));
    }

    // Sort dispatches by time
    dispatch_times.sort_by_key(|&(_, t)| t);

    let total_duration = start_all.elapsed();
    println!(
        "Completed {} requests in {:?}. Max observed concurrency: {}",
        num_requests,
        total_duration,
        max_observed_concurrency.load(Ordering::SeqCst)
    );

    // Verify concurrency cap was never violated
    assert!(
        max_observed_concurrency.load(Ordering::SeqCst) <= max_concurrency,
        "Observed concurrency {} exceeded maximum limit {}",
        max_observed_concurrency.load(Ordering::SeqCst),
        max_concurrency
    );

    // Verify inter-request dispatch spacing (allowing 3ms timer jitter)
    let min_allowed_gap = min_interval.saturating_sub(Duration::from_millis(3));
    for i in 1..dispatch_times.len() {
        let gap = dispatch_times[i].1.saturating_duration_since(dispatch_times[i - 1].1);
        assert!(
            gap >= min_allowed_gap,
            "Gap between dispatch {} and {} was {:?}, expected at least {:?}",
            i - 1,
            i,
            gap,
            min_allowed_gap
        );
    }

    let expected_min_total = min_interval * (num_requests as u32 - 1);
    assert!(
        total_duration >= expected_min_total.saturating_sub(Duration::from_millis(10)),
        "Total duration {:?} was less than expected minimum {:?}",
        total_duration,
        expected_min_total
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_request_gate_cancellation_resilience() {
    let gate = Arc::new(RequestGate::new(ThrottleConfig {
        min_interval: Duration::from_millis(20),
        max_concurrency: 2,
    }));

    // Occupy all 2 permits
    let p1 = gate.acquire().await;
    let p2 = gate.acquire().await;

    // Launch a task that will be cancelled while waiting on semaphore
    let gate_c = gate.clone();
    let cancel_handle = tokio::spawn(async move {
        let _ = gate_c.acquire().await;
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    cancel_handle.abort();
    let _ = cancel_handle.await;

    // Release one permit
    drop(p1);

    // A new acquire should succeed without deadlock
    let res = tokio::time::timeout(Duration::from_millis(100), gate.acquire()).await;
    assert!(res.is_ok(), "Acquiring permit after cancelled task must succeed");

    drop(p2);
    let p3 = res.unwrap();
    drop(p3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_concurrent_scraper_lock_and_ai_summary_no_contention() {
    let auth = Arc::new(MoodleAuth::new());
    let scraper_state = Arc::new(tokio::sync::Mutex::new(Some(MoodleScraper::new(auth.clone()))));
    let ai_state = Arc::new(tokio::sync::Mutex::new(Some(muster_lib::AiConfig {
        api_key: "test-key".to_string(),
        api_url: "http://127.0.0.1:9".to_string(),
        model: "test-model".to_string(),
    })));

    let num_callers = 50;
    let mut handles = Vec::new();

    for i in 0..num_callers {
        let sc_state = scraper_state.clone();
        let ai_st = ai_state.clone();

        handles.push(tokio::spawn(async move {
            for _ in 0..20 {
                // Emulate lib.rs pattern: lock briefly, clone/extract, drop lock
                let scraper = {
                    let guard = sc_state.lock().await;
                    guard.as_ref().cloned().expect("scraper present")
                };

                let (key, url, md) = {
                    let ai_guard = ai_st.lock().await;
                    let c = ai_guard.as_ref().unwrap();
                    (c.api_key.clone(), c.api_url.clone(), c.model.clone())
                };

                // With locks dropped, simulate long async call
                tokio::time::sleep(Duration::from_millis(1)).await;
                assert_eq!(key, "test-key");
                assert_eq!(url, "http://127.0.0.1:9");
                assert_eq!(md, "test-model");
                let _ = scraper;
            }
            i
        }));
    }

    let timeout_res = tokio::time::timeout(Duration::from_secs(5), async {
        for h in handles {
            h.await.unwrap();
        }
    }).await;

    assert!(timeout_res.is_ok(), "All concurrent callers must complete rapidly without deadlock");
}
