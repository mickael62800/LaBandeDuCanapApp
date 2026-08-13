use super::*;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn flushes_when_batch_full() {
    let flushed = Arc::new(Mutex::new(Vec::<Vec<u32>>::new()));
    let flushed_clone = flushed.clone();

    let writer = BatchWriter::spawn(
        "test",
        BatchWriterConfig {
            flush_interval: Duration::from_secs(60), // tres long pour ne pas trigger
            max_batch_size: 3,
            channel_capacity: 100,
        },
        move |batch: Vec<u32>| {
            let store = flushed_clone.clone();
            async move {
                store.lock().await.push(batch);
                Ok(())
            }
        },
    );

    for i in 0..5u32 {
        assert!(writer.try_send(i));
    }

    // Laisser le flusher tourner un peu
    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard = flushed.lock().await;
    assert_eq!(guard.len(), 1, "un batch de 3 doit avoir ete flush");
    assert_eq!(guard[0], vec![0, 1, 2]);
}

#[tokio::test]
async fn flushes_on_interval() {
    let flushed = Arc::new(Mutex::new(Vec::<Vec<u32>>::new()));
    let flushed_clone = flushed.clone();

    let writer = BatchWriter::spawn(
        "test",
        BatchWriterConfig {
            flush_interval: Duration::from_millis(50),
            max_batch_size: 1000, // tres grand pour ne pas trigger par taille
            channel_capacity: 100,
        },
        move |batch: Vec<u32>| {
            let store = flushed_clone.clone();
            async move {
                store.lock().await.push(batch);
                Ok(())
            }
        },
    );

    writer.try_send(42);
    writer.try_send(43);

    tokio::time::sleep(Duration::from_millis(150)).await;

    let guard = flushed.lock().await;
    assert!(
        !guard.is_empty(),
        "le tick doit avoir flush le batch partiel"
    );
    let all: Vec<u32> = guard.iter().flatten().copied().collect();
    assert_eq!(all, vec![42, 43]);
}

#[tokio::test]
async fn drains_on_channel_close() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let writer = BatchWriter::spawn(
        "test",
        BatchWriterConfig {
            flush_interval: Duration::from_secs(60),
            max_batch_size: 1000,
            channel_capacity: 100,
        },
        move |batch: Vec<u32>| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(batch.len(), Ordering::SeqCst);
                Ok(())
            }
        },
    );

    writer.try_send(1);
    writer.try_send(2);
    writer.try_send(3);

    drop(writer);

    // Attendre que le flusher draine et exit
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn flush_error_does_not_stop_loop() {
    // Verifie que si flush_fn retourne Err, le flusher continue a
    // accepter des nouveaux items et ne panique pas. Les entries du
    // batch qui a echoue sont perdues (at-most-once) mais les suivants
    // sont quand meme traites.
    let call_count = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();
    let sc = success_count.clone();

    let writer = BatchWriter::spawn(
        "test",
        BatchWriterConfig {
            flush_interval: Duration::from_millis(30),
            max_batch_size: 2,
            channel_capacity: 100,
        },
        move |batch: Vec<u32>| {
            let cc = cc.clone();
            let sc = sc.clone();
            async move {
                let n = cc.fetch_add(1, Ordering::SeqCst);
                // Premier batch : erreur simulée. Suivants : succès.
                if n == 0 {
                    Err("simulated db error".to_string())
                } else {
                    sc.fetch_add(batch.len(), Ordering::SeqCst);
                    Ok(())
                }
            }
        },
    );

    // Premier batch (2 items) → va échouer
    writer.try_send(1);
    writer.try_send(2);
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Second batch (2 items) → doit réussir malgré l'échec précédent
    writer.try_send(3);
    writer.try_send(4);
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Au moins 2 appels à flush_fn, et le second a bien traité 2 items
    assert!(
        call_count.load(Ordering::SeqCst) >= 2,
        "flush_fn doit etre rappele apres un echec"
    );
    assert_eq!(
        success_count.load(Ordering::SeqCst),
        2,
        "second batch doit etre persiste"
    );
}

#[tokio::test]
async fn try_send_returns_false_when_channel_full() {
    // Config extrême : capacité 1, max_batch_size énorme, interval très long
    // → le canal va se remplir avant que le flusher puisse drainer.
    let writer = BatchWriter::spawn(
        "test",
        BatchWriterConfig {
            flush_interval: Duration::from_secs(60),
            max_batch_size: 1_000,
            channel_capacity: 1,
        },
        move |_batch: Vec<u32>| async move {
            // Flush extrêmement lent pour laisser le canal saturer
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(())
        },
    );

    // Bourre le canal — au moins un send doit echouer
    let mut dropped = 0;
    for i in 0..20u32 {
        if !writer.try_send(i) {
            dropped += 1;
        }
    }
    assert!(
        dropped > 0,
        "au moins un try_send doit renvoyer false quand le canal est plein"
    );
}
