//! Building several pipelines at once.
//!
//! A pipeline costs the driver anywhere from a few to a few hundred milliseconds to compile when its cache has
//! nothing for it (the first start after a driver or shader change), and a cache that fills a whole variant table
//! at once would pay that serially. `wgpu::Device` is `Send + Sync` and its pipeline creation may be called from
//! several threads at the same time, so the variants are built on scoped worker threads instead, one per core,
//! each taking the next missing variant.
//!
//! `BLACKBOX_SERIAL_PIPELINES=1` builds them one after another on the calling thread, for comparison or for a
//! driver that misbehaves with concurrent compiles.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Whether builds may use worker threads (see the module docs). Read once.
fn parallel() -> bool {
    static PARALLEL: OnceLock<bool> = OnceLock::new();
    *PARALLEL.get_or_init(|| {
        let serial = std::env::var("BLACKBOX_SERIAL_PIPELINES").is_ok_and(|v| !v.is_empty() && v != "0");
        !serial
    })
}

/// How many workers to build `jobs` things with: one per core, never more than there are jobs.
fn workers(jobs: usize) -> usize {
    if !parallel() {
        return 1;
    }
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    cores.min(jobs)
}

/// `build` applied to every key, in the keys' order, spread over worker threads.
pub(super) fn build<K, T, F>(keys: &[K], build: F) -> Vec<T>
where
    K: Sync,
    T: Send,
    F: Fn(&K) -> T + Sync,
{
    let workers = workers(keys.len());
    if workers <= 1 {
        return keys.iter().map(&build).collect();
    }
    let next = AtomicUsize::new(0);
    let mut built: Vec<(usize, T)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(key) = keys.get(i) else { break };
                        done.push((i, build(key)));
                    }
                    done
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    });
    built.sort_unstable_by_key(|(i, _)| *i);
    built.into_iter().map(|(_, t)| t).collect()
}

/// [`build`] for a fixed-size table, such as one pipeline per blend mode.
pub(super) fn build_array<K, T, F, const N: usize>(keys: [K; N], f: F) -> [T; N]
where
    K: Sync,
    T: Send,
    F: Fn(&K) -> T + Sync,
{
    build(&keys, f).try_into().unwrap_or_else(|_: Vec<T>| unreachable!("one result per key"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_keep_the_keys_order() {
        let keys: Vec<u32> = (0..100).collect();
        let squares = build(&keys, |k| k * k);
        assert_eq!(squares, keys.iter().map(|k| k * k).collect::<Vec<_>>());
    }

    #[test]
    fn an_empty_or_single_list_needs_no_thread() {
        assert!(build(&[] as &[u8], |k| *k).is_empty());
        assert_eq!(build(&[7u8], |k| *k + 1), vec![8]);
        assert_eq!(build_array([1, 2, 3, 4], |k| k * 10), [10, 20, 30, 40]);
    }
}
