use mouse_rs::{types::keys::Keys, Mouse as MouseRs};
use rand::Rng;

use crate::invoke;

pub trait MouseController {
    fn random_xy(&self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> (i32, i32);
    fn move_to(&self, x: i32, y: i32);
    fn order(&self, setting: &invoke::gui::Mouse);
}

#[derive(Default)]
pub struct Mouse {}

impl Mouse {
    pub fn random_xy(&self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> (i32, i32) {
        let mut rng = rand::thread_rng();

        let x = rng.gen_range(min_x..max_x);
        let y = rng.gen_range(min_y..max_y);
        (x, y)
    }

    pub fn move_to(&self, x: i32, y: i32) {
        let rs = MouseRs::new();
        rs.move_to(x, y).unwrap()
    }

    pub fn order(&self, setting: &invoke::gui::Mouse) {
        let (min_x, min_y, max_x, max_y) = {
            (
                setting.start_x as i32,
                setting.start_y as i32,
                setting.end_x as i32,
                setting.end_y as i32,
            )
        };
        let (x, y) = self.random_xy(min_x, min_y, max_x, max_y);

        let rs = MouseRs::new();
        rs.move_to(x, y).unwrap();
        rs.click(&Keys::LEFT).unwrap()
    }
}

impl MouseController for Mouse {
    fn random_xy(&self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> (i32, i32) {
        Mouse::random_xy(self, min_x, min_y, max_x, max_y)
    }

    fn move_to(&self, x: i32, y: i32) {
        Mouse::move_to(self, x, y)
    }

    fn order(&self, setting: &invoke::gui::Mouse) {
        Mouse::order(self, setting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_xy() {
        let interval = 10;
        let min_ms = (interval as f32 * 0.5 * 1000f32) as u32;
        let max_ms = (interval as f32 * 1.5 * 1000f32) as u32;
        let mut rng = rand::thread_rng();

        let count = 10000;
        let mut numbers = Vec::new();
        for _ in 0..count {
            let number = rng.gen_range(min_ms..max_ms);
            numbers.push(number);
        }

        // 平均を求める
        let sum = numbers.iter().sum::<u32>();
        let average = sum as f32 / count as f32;
        println!("average: {}", average);
        println!("min: {}, max: {}", min_ms, max_ms);
    }
}
