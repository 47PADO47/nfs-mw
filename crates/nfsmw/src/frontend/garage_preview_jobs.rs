//! One decoder and one replaceable next request; dropping a scene never waits for disk work.

use anyhow::Result;
use std::thread::JoinHandle;

pub(super) struct Jobs<Q, R> {
    pub desired: Option<Q>,
    active: Option<(Q, JoinHandle<Result<R>>)>,
}

impl<Q, R> Default for Jobs<Q, R> {
    fn default() -> Self {
        Self { desired: None, active: None }
    }
}

impl<Q: Clone + Send + 'static, R: Send + 'static> Jobs<Q, R> {
    pub fn needs_start(&self) -> bool {
        self.active.is_none() && self.desired.is_some()
    }

    pub fn request(&mut self, request: Q) {
        self.desired = Some(request);
    }

    pub fn cancel(&mut self) {
        self.desired = None;
    }

    pub fn start(&mut self, load: impl FnOnce(Q) -> Result<R> + Send + 'static) -> Result<()> {
        if self.active.is_some() {
            return Ok(());
        }
        let Some(request) = self.desired.clone() else { return Ok(()) };
        let work = request.clone();
        let worker = std::thread::Builder::new().name("my-cars-preview".into()).spawn(move || load(work))?;
        self.active = Some((request, worker));
        Ok(())
    }

    pub fn poll(&mut self, matches: impl FnOnce(&Q, &Q) -> bool) -> Option<(Q, Result<R>)> {
        if self.active.as_ref().is_none_or(|(_, worker)| !worker.is_finished()) {
            return None;
        }
        let (started, worker) = self.active.take().expect("finished preview worker");
        let result = worker.join().unwrap_or_else(|_| Err(anyhow::anyhow!("Car preview loader panicked")));
        let latest = self.desired.as_ref()?;
        if !matches(&started, latest) {
            return None;
        }
        Some((self.desired.take().expect("latest request"), result))
    }
}

#[cfg(test)]
#[path = "garage_preview_jobs_tests.rs"]
mod tests;
