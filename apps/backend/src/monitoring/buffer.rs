use crate::hardware::GpuSample;

pub struct RingBuffer {
    capacity: usize,
    data: Vec<GpuSample>,
    head: usize,
    count: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: Vec::with_capacity(capacity),
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, sample: GpuSample) {
        if self.count < self.capacity {
            self.data.push(sample);
            self.count += 1;
        } else {
            self.data[self.head] = sample;
            self.head = (self.head + 1) % self.capacity;
        }
    }

    pub fn get_latest(&self) -> Option<&GpuSample> {
        if self.count == 0 {
            return None;
        }
        let idx = if self.count < self.capacity {
            self.count - 1
        } else {
            (self.head + self.capacity - 1) % self.capacity
        };
        self.data.get(idx)
    }

    pub fn get_range(&self, count: usize) -> Vec<&GpuSample> {
        let take = count.min(self.count);
        let mut result = Vec::with_capacity(take);

        let start = if self.count < self.capacity {
            0
        } else {
            self.head
        };

        for i in 0..take {
            let idx = (start + self.count - take + i) % self.capacity;
            if let Some(sample) = self.data.get(idx) {
                result.push(sample);
            }
        }

        result
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.head = 0;
        self.count = 0;
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::GpuSample;

    fn make_sample(gpu_id: &str, temp: f64) -> GpuSample {
        GpuSample {
            gpu_id: gpu_id.to_string(),
            timestamp: 0,
            temperature_celsius: temp,
            core_clock_mhz: 0.0,
            memory_clock_mhz: 0.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            fan_speed_percent: 0.0,
            fan_rpm: 0,
            power_watts: 0.0,
            max_power_limit_watts: None,
            core_voltage_mv: 0.0,
            core_utilization_percent: 0.0,
            core_utilization_available: true,
            memory_utilization_percent: 0.0,
        }
    }

    #[test]
    fn test_push_and_get_latest() {
        let mut buf = RingBuffer::new(100);
        assert!(buf.is_empty());

        buf.push(make_sample("gpu-0", 45.0));
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.get_latest().unwrap().temperature_celsius, 45.0);

        buf.push(make_sample("gpu-0", 50.0));
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.get_latest().unwrap().temperature_celsius, 50.0);
    }

    #[test]
    fn test_ring_wraparound() {
        let mut buf = RingBuffer::new(3);
        buf.push(make_sample("gpu-0", 1.0));
        buf.push(make_sample("gpu-0", 2.0));
        buf.push(make_sample("gpu-0", 3.0));
        buf.push(make_sample("gpu-0", 4.0));

        assert_eq!(buf.len(), 3);
        let range = buf.get_range(3);
        assert_eq!(range[0].temperature_celsius, 2.0);
        assert_eq!(range[1].temperature_celsius, 3.0);
        assert_eq!(range[2].temperature_celsius, 4.0);
    }

    #[test]
    fn test_get_range() {
        let mut buf = RingBuffer::new(100);
        for i in 0..10 {
            buf.push(make_sample("gpu-0", i as f64));
        }

        let range = buf.get_range(3);
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].temperature_celsius, 7.0);
        assert_eq!(range[1].temperature_celsius, 8.0);
        assert_eq!(range[2].temperature_celsius, 9.0);
    }
}
