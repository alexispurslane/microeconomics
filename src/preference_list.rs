use crate::items::discretes::Goal;
use crate::items::discretes::Item;
use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum GoalData {
    Regular {
        goal: Goal,
        time_required: i32,
        time: i32,
        id: i32,
    },
    Satisfaction {
        goal: Goal,
        units_required: i32,
        units: i32,
        id: i32,
    },
    RegularSatisfaction {
        goal: Goal,
        time_required: i32,
        time: i32,
        units_required: i32,
        units: i32,
        id: i32,
    },
}

impl GoalData {
    pub fn get_goal(&self) -> Goal {
        match self {
            &GoalData::Regular { goal, .. }
            | &GoalData::Satisfaction { goal, .. }
            | &GoalData::RegularSatisfaction { goal, .. } => goal,
        }
    }
}

pub struct GoalWrapper {
    comparator: Rc<Box<dyn Fn(&GoalData, &GoalData) -> Ordering>>,
    goal: GoalData,
}

impl PartialOrd for GoalWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GoalWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.goal == other.goal
    }
}

impl Eq for GoalWrapper {}

impl Ord for GoalWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.comparator)(&self.goal, &other.goal)
    }
}

pub type PreferenceList = HashMap<Item, BinaryHeap<GoalWrapper>>;

pub struct Actor {
    name: String,
    recurring_goals: Vec<GoalData>,
    preference_list: PreferenceList,
    satisfactions: HashMap<Goal, Vec<Item>>,
    comparator: Rc<Box<dyn Fn(&GoalData, &GoalData) -> Ordering>>,
}

impl Actor {
    pub fn new(name: &str, c: Rc<Box<dyn Fn(&GoalData, &GoalData) -> Ordering>>) -> Self {
        Actor {
            name: name.to_owned(),
            recurring_goals: vec![],
            preference_list: HashMap::new(),
            comparator: c,
            satisfactions: HashMap::new(),
        }
    }

    pub fn add_goal(&mut self, goal: GoalData) {
        let actual_goal = goal.get_goal();
        if let Some(effected_entries) = self.satisfactions.get(&actual_goal) {
            for item in effected_entries.iter() {
                {
                    let c = self.comparator.clone();
                    let ordered_goal = GoalWrapper {
                        comparator: c,
                        goal: goal,
                    };
                    let mut goals = BinaryHeap::new();
                    goals.push(ordered_goal);
                    self.preference_list
                        .entry(*item)
                        .or_insert(BinaryHeap::new())
                        .append(&mut goals);
                }
            }
        }
        match goal {
            GoalData::Regular { .. } | GoalData::RegularSatisfaction { .. } => {
                self.recurring_goals.push(goal.clone());
            }
            _ => {}
        }
    }

    pub fn remove_goal(&mut self, actual_goal: Goal) {
        if let Some(effected_entries) = self.satisfactions.get(&actual_goal) {
            for item in effected_entries.iter() {
                {
                    if self.preference_list.contains_key(&item) {
                        let mut new = BinaryHeap::new();
                        self.preference_list
                            .get(&item)
                            .map(|goals: &BinaryHeap<GoalWrapper>| {
                                for og in goals.into_iter() {
                                    if og.goal.get_goal() != actual_goal {
                                        new.push(GoalWrapper {
                                            comparator: self.comparator.clone(),
                                            goal: og.goal,
                                        });
                                    }
                                }
                            });
                        *self.preference_list.get_mut(&item).unwrap() = new;
                    }
                }
            }
        }
        let index = self
            .recurring_goals
            .iter()
            .position(|w: &GoalData| w.get_goal() == actual_goal);
        if let Some(i) = index {
            self.recurring_goals.remove(i);
        }
    }

    pub fn use_item(&mut self, item: Item) -> Option<GoalData> {
        if let Some(goals) = self.preference_list.get_mut(&item) {
            if let Some(wrapper) = goals.peek() {
                let highest_valued_goal: GoalData = wrapper.goal;
                match highest_valued_goal {
                    GoalData::Regular { goal, .. } => {
                        self.remove_goal(goal);
                        Some(highest_valued_goal)
                    }
                    GoalData::Satisfaction {
                        goal,
                        units_required,
                        mut units,
                        ..
                    } => {
                        units += 1;
                        if units >= units_required {
                            self.remove_goal(goal);
                            Some(highest_valued_goal)
                        } else {
                            None
                        }
                    }
                    GoalData::RegularSatisfaction {
                        goal,
                        units_required,
                        mut units,
                        ..
                    } => {
                        units += 1;
                        if units >= units_required {
                            self.remove_goal(goal);
                            Some(highest_valued_goal)
                        } else {
                            None
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn add_satisfaction_entry(&mut self, goal: Goal, item: Item) {
        self.satisfactions
            .entry(goal)
            .or_insert(vec![item])
            .push(item);
    }

    pub fn get_best_goal(&self, item: Item) -> Option<Goal> {
        self.preference_list
            .get(&item)
            .and_then(|goals| goals.peek())
            .map(|og| og.goal.get_goal())
    }

    pub fn compare_item_values(&self, a: Item, b: Item) -> Option<Ordering> {
        self.get_best_goal(a)
            .and_then(|a_g| self.get_best_goal(b).map(|b_g| a_g.cmp(&b_g)))
    }
}
