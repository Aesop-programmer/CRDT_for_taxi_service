
use std::time::{Duration, Instant};
use r2r::uuid::timestamp;
use serde::de;
use time::convert::Second;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub car_id: u32,
    pub busy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoTable{
    pub vehicle_state: Vec<VehicleState>, // current exist car
    pub task: Vec<Task>, // current task
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: u32,
    pub cur_location: u32, // todo fill in location
    pub des_location: u32, // todo fill in location
    pub timestamp: u64,
    pub assigned_car: u32, // 0 indicates no car is assigned
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTaxi {
    pub task_id: u32,
    pub cur_location: u32, // todo fill in location
    pub des_location: u32, // todo fill in location
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequireTask {
    pub task_id: u32,
    pub car_id: u32,
    pub timestamp: u64,
}

impl VehicleState {
    fn new(car_id: u32, busy: bool) -> Self {
        VehicleState {
            car_id,
            busy,
        }
    }
}

impl InfoTable{
    pub fn new() -> Self {
        Self {
            vehicle_state: Vec::new(),
            task: Vec::new(),
        }
    }

    pub fn add_vehicle(&mut self, car_id: u32, busy: bool) {
        for i in 0..self.vehicle_state.len() {
            if self.vehicle_state[i].car_id == car_id {
                return
            }
        }
        self.vehicle_state.push(VehicleState::new(car_id, busy));
    }

    pub fn called_taxi(&mut self, request: CallTaxi) {
        let mut assigned_car = 0;
        let mut timestamp = 0;
        for i in 0..self.task.len() {
            if self.task[i].task_id == request.task_id {
                return
            }
        }
        self.task.push(Task {
            task_id: request.task_id,
            cur_location: request.cur_location,
            des_location: request.des_location,
            timestamp,
            assigned_car,
        });
    }

    pub fn merge_task(&mut self, task: RequireTask) {
        for i in 0..self.task.len() {
            if self.task[i].task_id == task.task_id {
                if self.task[i].assigned_car == 0 {
                    self.task[i].assigned_car = task.car_id;
                    self.task[i].timestamp = task.timestamp;
                }
                else {
                    if self.task[i].timestamp > task.timestamp {
                        self.task[i].assigned_car = task.car_id;
                        self.task[i].timestamp = task.timestamp;
                    }
                }
            }
            break;
        }
    }

}




