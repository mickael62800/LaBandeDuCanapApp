use super::*;

#[tokio::test]
async fn test_acquire_within_limits() {
    let limiter = InferenceRateLimiter::new(4, 100);
    let permit = limiter.acquire().await;
    assert!(permit.is_ok());
}

#[tokio::test]
async fn test_concurrent_limit() {
    let limiter = InferenceRateLimiter::new(2, 0);
    let _p1 = limiter.acquire().await.unwrap();
    let _p2 = limiter.acquire().await.unwrap();
    assert!(limiter.semaphore.try_acquire().is_err());
}

#[tokio::test]
async fn test_rate_limit_zero_unlimited() {
    let limiter = InferenceRateLimiter::new(10, 0);
    for _ in 0..20 {
        assert!(limiter.acquire().await.is_ok());
    }
}

#[tokio::test]
async fn test_token_bucket_depletes() {
    let limiter = InferenceRateLimiter::new(100, 1);
    for _ in 0..5 {
        assert!(limiter.acquire().await.is_ok());
    }
    assert!(limiter.acquire().await.is_err());
}

#[tokio::test]
async fn test_clone_shares_state() {
    // Les Arc<_> internes doivent faire que l'etat est partage apres clone.
    let limiter = InferenceRateLimiter::new(1, 0);
    let clone = limiter.clone();
    let _p1 = limiter.acquire().await.unwrap();
    // clone doit voir que le semaphore est maintenant vide
    assert!(clone.semaphore.try_acquire().is_err());
}

#[tokio::test]
async fn test_acquire_succeeds_after_permit_dropped() {
    let limiter = InferenceRateLimiter::new(1, 0);
    {
        let _p1 = limiter.acquire().await.unwrap();
        // Semaphore plein ici.
    } // p1 drop → permit libere.
      // Doit reussir immediatement (pas de timeout).
    assert!(limiter.acquire().await.is_ok());
}

#[tokio::test]
async fn test_refill_tokens_restores_capacity_after_wait() {
    // max_per_sec=10 → max_tokens=50 (burst 5 secs).
    let limiter = InferenceRateLimiter::new(100, 10);
    // Drain les 50 tokens initiaux.
    for _ in 0..50 {
        assert!(limiter.acquire().await.is_ok());
    }
    // 51e doit echouer (tokens = 0).
    assert!(limiter.acquire().await.is_err());

    // Attendre ~200ms => refill = (0.2 * 10) as u64 = 2 tokens.
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    // On doit pouvoir acquire au moins 1 fois apres refill.
    assert!(limiter.acquire().await.is_ok());
}

#[tokio::test]
async fn test_acquire_via_timeout_path_succeeds() {
    // Couvre la branche `Err(_) => timeout + acquire.await` quand try_acquire
    // echoue mais le permit se libere avant le timeout de 5s.
    let limiter = InferenceRateLimiter::new(1, 0);
    // Acquiert via un OwnedSemaphorePermit pour pouvoir le move dans une task.
    let sem = limiter.semaphore.clone();
    let owned_permit = sem.try_acquire_owned().unwrap();

    // Task qui libere le permit apres 80ms.
    let release = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
        drop(owned_permit);
    });

    // limiter.acquire() doit : try_acquire fail → timeout path → acquire.await → Ok.
    let result = limiter.acquire().await;
    assert!(result.is_ok());
    release.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_acquire_timeout_returns_rate_limited_error() {
    // Couvre la branche `_ => Err(RateLimited(...))` du chemin timeout :
    // try_acquire echoue + tokio::time::timeout(5s) expire.
    // `start_paused = true` permet d'avancer le temps virtuellement.
    let limiter = InferenceRateLimiter::new(1, 0);
    let sem = limiter.semaphore.clone();
    // Garder le permit actif pour que try_acquire echoue.
    let _permit = sem.try_acquire_owned().unwrap();

    // Pin le future et avance le temps virtuel pour faire expirer le timeout de 5s.
    let acquire_fut = limiter.acquire();
    tokio::pin!(acquire_fut);

    // Poll une fois pour armer le timer interne.
    let result = tokio::select! {
        r = &mut acquire_fut => r,
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(6)) => {
            // Le timer de acquire (5s) devrait avoir expire avant ce sleep.
            acquire_fut.await
        }
    };
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DomainError::RateLimited(_)));
}

#[tokio::test]
async fn test_rate_limit_error_variant() {
    // Verifier que c'est bien RateLimited (pas Internal ou autre).
    let limiter = InferenceRateLimiter::new(100, 1);
    // Drain.
    for _ in 0..5 {
        let _ = limiter.acquire().await;
    }
    let err = limiter.acquire().await.unwrap_err();
    assert!(matches!(err, DomainError::RateLimited(_)));
}
