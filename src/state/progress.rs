use std::sync::atomic::AtomicU64;

#[derive(Debug, serde::Deserialize,serde::Serialize, Default, Copy, Clone)]
pub struct Progress {
    pub max: u64,
    pub value: u64,
    pub state: ProgressState,
}
impl Progress {
    // Increments and sets state
    pub fn increment_check(&mut self) -> &ProgressState {
        if self.value == 0 {
            self.state = ProgressState::Running;
        }

        self.value += 1;

        if self.value == self.max {
            self.state = ProgressState::Finished;
        }
        &self.state
    }
}
impl From<AtomicProgress> for Progress{
    fn from(ap: AtomicProgress) -> Self {
        let max  = ap.max();
        let value = ap.value();
        let state = ap.state();
        Self { max, value, state }
    }
}
impl From<&AtomicProgress> for Progress{
    fn from(ap: &AtomicProgress) -> Self {
        let max  = ap.max();
        let value = ap.value();
        let state = ap.state();
        Self { max, value, state }
    }
}

#[derive(Debug)]
pub struct AtomicProgress {
    state: AtomicProgressState,
    value: AtomicU64,
    max: AtomicU64,
}
impl AtomicProgress {
    pub fn new(max: u64) -> Self {
        Self {
            max: AtomicU64::new(max),
            ..Default::default()
        }
    }
    pub fn increment(&self) {
        self.value
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    // Increments and sets state automatically
    pub fn increment_check(&self) -> ProgressState {
        if self.value() == 0 {
            self.set_state(ProgressState::Running)
        }

        self.increment();

        if self.value() == self.max.load(std::sync::atomic::Ordering::Relaxed) {
            self.set_state(ProgressState::Finished);
        }
        self.state()
    }
    pub fn value(&self) -> u64 {
        self.value.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn state(&self) -> ProgressState {
        self.state.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn set_max(&self, value: u64) {
        self.max.store(value, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn reset(&self) {
        self.state
            .store(ProgressState::Stopped, std::sync::atomic::Ordering::Relaxed);
        self.value.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn max(&self)->u64{
        self.max.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn set_state(&self, value: ProgressState) {
        self.state
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }
}
impl Default for AtomicProgress {
    fn default() -> Self {
        Self {
            state: AtomicProgressState::new(ProgressState::Stopped),
            value: AtomicU64::new(0),
            max: AtomicU64::new(0),
        }
    }
}


#[atomic_enum::atomic_enum]
#[derive(PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub enum ProgressState {
    Running,
    #[default]
    Stopped,
    Finished,
}

