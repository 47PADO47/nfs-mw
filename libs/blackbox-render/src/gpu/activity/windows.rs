//! Windows PDH GPU Engine counters and GetThreadTimes, sampled once a second on an owned worker.
#![allow(unsafe_code, reason = "Windows owns the counter buffers and thread/query handles")]
use blackbox_gfx::WorkloadActivity;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::{Performance::*, Threading::*};
use windows::core::{PCWSTR, w};

#[derive(Clone, Copy)]
struct Snapshot {
    at: Instant,
    value: WorkloadActivity,
}
pub(super) struct Reader {
    state: Arc<Mutex<Option<Snapshot>>>,
    // Dropping this disconnects the worker's interruptible wait. No join/stall on the render thread.
    _stop: mpsc::Sender<()>,
}
impl Reader {
    pub(super) fn new() -> Option<Self> {
        // The renderer's owning thread is the app's main/update/submission thread.
        let tid = unsafe { GetCurrentThreadId() };
        let state = Arc::new(Mutex::new(None));
        let worker_state = Arc::clone(&state);
        let (stop, wait) = mpsc::channel();
        std::thread::Builder::new().name("render activity".into()).spawn(move || run(tid, worker_state, wait)).ok()?;
        Some(Self { state, _stop: stop })
    }
    pub(super) fn sample(&self) -> WorkloadActivity {
        // The render thread never blocks on telemetry, even for the tiny snapshot copy.
        let Ok(state) = self.state.try_lock() else { return WorkloadActivity::default() };
        let Some(latest) = *state else { return WorkloadActivity::default() };
        let age = latest.at.elapsed();
        if age > Duration::from_millis(2500) {
            return WorkloadActivity::default();
        }
        WorkloadActivity { age_ms: age.as_secs_f32() * 1000.0, ..latest.value }
    }
}
struct Thread(HANDLE);
impl Thread {
    fn time(&self) -> Option<u64> {
        let (mut creation, mut exit, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        // SAFETY: the owned thread handle is valid throughout the synchronous query.
        unsafe {
            GetThreadTimes(self.0, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
        }
        let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        ticks(kernel).checked_add(ticks(user))
    }
}
impl Drop for Thread {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

struct Query {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}
impl Query {
    fn new() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        // The English API resolves the counter on localized Windows installations as well.
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut result = Self { query, counter: PDH_HCOUNTER::default() };
        if unsafe {
            PdhAddEnglishCounterW(query, w!("\\GPU Engine(*)\\Utilization Percentage"), 0, &mut result.counter)
        } != 0
        {
            return None;
        }
        unsafe {
            PdhCollectQueryData(query);
        }
        Some(result)
    }
    fn sample(&self, pid: u32) -> Option<(f32, &'static str)> {
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            return None;
        }
        // Size and instance count can change between the two calls. Retry with a bounded allocation.
        for _ in 0..3 {
            // After a growth race, PDH does not guarantee the returned size. Query it afresh.
            let (mut bytes, mut count) = (0, 0);
            let status =
                unsafe { PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None) };
            if status != PDH_MORE_DATA {
                return None;
            }
            if bytes == 0 || bytes > 16 * 1024 * 1024 {
                return None;
            }
            // usize storage supplies the alignment required by the C structs and embedded UTF-16 strings.
            let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
            let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let capacity = bytes;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, Some(items))
            };
            if status == PDH_MORE_DATA {
                continue;
            }
            if status != 0 || count as usize > capacity as usize / size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() {
                return None;
            }
            // SAFETY: PDH succeeded and returned count structs in this live, aligned allocation.
            let rows = unsafe { std::slice::from_raw_parts(items, count as usize) };
            let values: Vec<_> = rows
                .iter()
                .filter_map(|item| {
                    if !matches!(item.FmtValue.CStatus, PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA)
                        || item.szName.is_null()
                    {
                        return None;
                    }
                    // PDH owns these terminated names inside buffer until this call returns.
                    Some((unsafe { item.szName.to_string().ok()? }, unsafe { item.FmtValue.Anonymous.doubleValue }))
                })
                .collect();
            return super::sample::peak(pid, values.iter().map(|(name, value)| (name.as_str(), *value)));
        }
        None
    }
}
impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}
fn run(tid: u32, state: Arc<Mutex<Option<Snapshot>>>, wait: mpsc::Receiver<()>) {
    let thread = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, tid).ok() }.map(Thread);
    let query = Query::new();
    let mut previous = thread.as_ref().and_then(Thread::time);
    let mut at = Instant::now();
    let mut sample = 0;
    loop {
        if !matches!(wait.recv_timeout(Duration::from_secs(1)), Err(mpsc::RecvTimeoutError::Timeout)) {
            return;
        }
        let gpu = query.as_ref().and_then(|query| query.sample(std::process::id()));
        let now = Instant::now();
        let current = thread.as_ref().and_then(Thread::time);
        let cpu = current
            .zip(previous)
            .and_then(|(a, b)| a.checked_sub(b))
            .and_then(|ticks| super::sample::cpu_percent(ticks, now.duration_since(at).as_secs_f64()));
        previous = current;
        at = now;
        sample += 1;
        let value = WorkloadActivity {
            main_thread_busy_percent: cpu,
            gpu_busy_percent: gpu.map(|v| v.0),
            gpu_engine: gpu.map_or("", |v| v.1),
            sample,
            age_ms: 0.0,
        };
        if let Ok(mut latest) = state.lock() {
            *latest = Some(Snapshot { at: now, value });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires live Windows thread accounting; serial activity validation"]
    fn active_owner_thread_and_blocked_owner_thread_are_distinguished() {
        let reader = Reader::new().expect("activity worker");
        let until = Instant::now() + Duration::from_millis(3100);
        let mut latest = WorkloadActivity::default();
        while Instant::now() < until {
            std::hint::black_box((0u64..10_000).fold(0u64, |a, b| a.wrapping_add(b)));
            latest = reader.sample();
        }
        eprintln!("active owner: {latest:?}");
        assert!(latest.main_thread_busy_percent.is_some_and(|v| v > 65.0));
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(1100));
            latest = reader.sample();
        }
        eprintln!("blocked owner: {latest:?}");
        assert!(latest.main_thread_busy_percent.is_some_and(|v| v < 25.0));
    }
}
