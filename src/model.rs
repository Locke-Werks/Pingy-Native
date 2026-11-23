//! Per-target history and the statistics shown beside each host.

use std::collections::VecDeque;

/// One echo attempt. `t_ms` is the send time on the process clock
/// (`clock::now_ms`), which is the same clock the graph renders against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub t_ms: f64,
    pub latency_ms: Option<f32>,
    pub ttl: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub current: Option<f32>,
    pub average: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub loss_percent: f32,
    pub sent: usize,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub id: u64,
    pub host: String,
    pub name: String,
    pub active: bool,
    /// Last resolve or socket failure, shown in place of stats.
    pub error: Option<String>,
    history: VecDeque<Sample>,
    capacity: usize,
}

impl Target {
    pub fn new(id: u64, host: String, name: String, capacity: usize) -> Self {
        Self {
            id,
            host,
            name,
            active: true,
            error: None,
            history: VecDeque::with_capacity(capacity),
            capacity: capacity.max(2),
        }
    }

    pub fn push(&mut self, s: Sample) {
        while self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(s);
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(2);
        while self.history.len() > self.capacity {
            self.history.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub fn history(&self) -> impl Iterator<Item = &Sample> {
        self.history.iter()
    }

    /// The newest `n` samples, oldest first.
    pub fn tail(&self, n: usize) -> impl Iterator<Item = &Sample> {
        self.history.iter().skip(self.history.len().saturating_sub(n))
    }

    pub fn stats(&self) -> Stats {
        let mut st = Stats {
            sent: self.history.len(),
            ..Stats::default()
        };
        if self.history.is_empty() {
            return st;
        }
        let mut ok = 0usize;
        let mut sum = 0.0f64;
        for s in &self.history {
            if let Some(l) = s.latency_ms {
                ok += 1;
                sum += f64::from(l);
                st.min = Some(st.min.map_or(l, |m| m.min(l)));
                st.max = Some(st.max.map_or(l, |m| m.max(l)));
            }
        }
        if ok > 0 {
            st.average = Some((sum / ok as f64) as f32);
        }
        st.current = self.history.back().and_then(|s| s.latency_ms);
        st.loss_percent = 100.0 * (self.history.len() - ok) as f32 / self.history.len() as f32;
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: f64, l: Option<f32>) -> Sample {
        Sample { t_ms: t, latency_ms: l, ttl: None }
    }

    #[test]
    fn empty_target_has_no_stats() {
        let t = Target::new(1, "h".into(), "n".into(), 10);
        let st = t.stats();
        assert_eq!(st.current, None);
        assert_eq!(st.average, None);
        assert_eq!(st.loss_percent, 0.0);
    }

    #[test]
    fn stats_ignore_timeouts_and_count_them_as_loss() {
        let mut t = Target::new(1, "h".into(), "n".into(), 10);
        t.push(s(0.0, Some(10.0)));
        t.push(s(1.0, None));
        t.push(s(2.0, Some(30.0)));
        t.push(s(3.0, None));
        let st = t.stats();
        assert_eq!(st.min, Some(10.0));
        assert_eq!(st.max, Some(30.0));
        assert_eq!(st.average, Some(20.0));
        assert_eq!(st.current, None);
        assert_eq!(st.loss_percent, 50.0);
    }

    #[test]
    fn ring_drops_oldest() {
        let mut t = Target::new(1, "h".into(), "n".into(), 3);
        for i in 0..5 {
            t.push(s(f64::from(i), Some(i as f32)));
        }
        let v: Vec<f64> = t.history().map(|x| x.t_ms).collect();
        assert_eq!(v, vec![2.0, 3.0, 4.0]);
        t.set_capacity(2);
        let v: Vec<f64> = t.history().map(|x| x.t_ms).collect();
        assert_eq!(v, vec![3.0, 4.0]);
    }

    #[test]
    fn tail_returns_newest_in_order() {
        let mut t = Target::new(1, "h".into(), "n".into(), 10);
        for i in 0..5 {
            t.push(s(f64::from(i), None));
        }
        let v: Vec<f64> = t.tail(2).map(|x| x.t_ms).collect();
        assert_eq!(v, vec![3.0, 4.0]);
        assert_eq!(t.tail(50).count(), 5);
    }
}
