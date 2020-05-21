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
    pub fn is_recurring(&self) -> bool {
        match self {
            &GoalData::Satisfaction { .. } => false,
            _ => true,
        }
    }
}

pub struct GoalWrapper {
    comparator: Box<dyn Fn(&GoalData, &GoalData) -> Ordering>,
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

pub struct GoalHierarchy(HashMap<Goal, usize>);
impl GoalHierarchy {
    fn new(goals: Vec<Goal>) -> Self {
        let mut hm = HashMap::new();
        for (i, goal) in goals.into_iter().enumerate() {
            hm.insert(goal, i);
        }
        GoalHierarchy(hm)
    }
}

pub struct Actor {
    name: String,
    recurring_goals: HashMap<Goal, GoalData>,
    preference_list: PreferenceList,
    satisfactions: HashMap<Goal, Vec<Item>>,
    goal_hierarchy: Rc<GoalHierarchy>,
}

impl Actor {
    /// Construct a new actor. Does some housekeeping to make construction easier.
    ///
    /// # Arguments
    ///
    /// * `name` - actor's name, for printout results
    /// * `hierarchy` - list of actor's valued ends as `GoalData` so that they can also be added to other places.
    ///
    pub fn new(name: &str, hierarchy: Vec<GoalData>) -> Self {
        let mut this = Actor {
            name: name.to_owned(),
            recurring_goals: HashMap::new(),
            preference_list: HashMap::new(),
            satisfactions: HashMap::new(),
            goal_hierarchy: Rc::new(GoalHierarchy::new(
                hierarchy.iter().map(|x| x.get_goal()).collect(),
            )),
        };
        for (i, goal) in hierarchy.into_iter().enumerate() {
            this.add_goal(goal, i);
            if goal.is_recurring() {
                this.recurring_goals.insert(goal.get_goal(), goal);
            }
        }
        this
    }

    /// Adds a goal to all of the BinaryHeaps for all of the items that can satisfy it (sorted).
    ///
    /// # Arguments
    ///
    /// * `goal` - `GoalData` of what's to be added
    /// * `location` - the location for it to be inserted into the hierarchy of ends/values
    ///
    pub fn add_goal(&mut self, goal: GoalData, location: usize) {
        let actual_goal = goal.get_goal();
        if let Some(effected_entries) = self.satisfactions.get(&actual_goal) {
            for item in effected_entries.iter() {
                {
                    let gh = self.goal_hierarchy.clone();
                    let ordered_goal = GoalWrapper {
                        comparator: Box::new(move |x: &GoalData, y: &GoalData| {
                            let xval = gh.0.get(&x.get_goal());
                            let yval = gh.0.get(&y.get_goal());
                            xval.and_then(|x| yval.map(|y| x.cmp(y)))
                                .unwrap_or(Ordering::Equal)
                        }),
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
        if goal.is_recurring() {
            self.recurring_goals.insert(goal.get_goal(), goal.clone());
        }
        Rc::get_mut(&mut self.goal_hierarchy)
            .unwrap()
            .0
            .insert(goal.get_goal(), location);
    }

    /// Removes any goal in the entire list of goals this actor has.
    ///
    /// # Arguments
    ///
    /// * `actual_goal` - The goal (not `GoalData` or `GoalWrapper`) to remove
    ///
    /// # Notes
    ///
    /// Since items are always used for the highest-valued goal which they can
    /// satisfy (and thus the base node in the BinaryHeap), `pop()` would
    /// suffice in the small case. That would be ideal because it would be very
    /// fast. However, for goals that can be satisfied by multiple items, which
    /// might be the highest valued goal that can be satisfied by some items but
    /// not by others, we need to be more complex. This method is an extreme
    /// performance basket-case and should basically never be used unless
    /// absolutely totally necessary
    ///
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
                                        let gh = self.goal_hierarchy.clone();
                                        new.push(GoalWrapper {
                                            comparator: Box::new(
                                                move |x: &GoalData, y: &GoalData| {
                                                    let xval = gh.0.get(&x.get_goal());
                                                    let yval = gh.0.get(&y.get_goal());
                                                    xval.and_then(|x| yval.map(|y| x.cmp(y)))
                                                        .unwrap_or(Ordering::Equal)
                                                },
                                            ),
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
        self.recurring_goals.remove(&actual_goal);
        Rc::get_mut(&mut self.goal_hierarchy)
            .unwrap()
            .0
            .remove(&actual_goal);
    }

    /// Uses an item to satisfy the most valued goal it can satisfy.
    ///
    /// # Arguments
    ///
    /// * `item` - `Item` to use
    ///
    /// # Notes
    ///
    /// Doesn't update recurring goals. See `tick`.
    ///
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

    /// Add an item to the list of items that can satisfy a given goal.
    ///
    /// # Arguments
    ///
    /// * `goal` - the goal that can be satisfied with this item
    /// * `item` - the item that can satisfy this goal
    ///
    pub fn add_satisfaction_entry(&mut self, goal: Goal, item: Item) {
        self.satisfactions
            .entry(goal)
            .or_insert(vec![item])
            .push(item);
    }

    /// Get the highest-valued goal which can be satisfied with this item
    ///
    /// # Arguments
    ///
    /// * `item` - the item
    ///
    pub fn get_best_goal(&self, item: Item) -> Option<Goal> {
        self.preference_list
            .get(&item)
            .and_then(|goals| goals.peek())
            .map(|og| og.goal.get_goal())
    }

    /// Compare two items to see which is more valuable based on the goals it can satisfy
    ///
    /// # Arguments
    ///
    /// * `a` - first item
    /// * `b` - second item
    ///
    pub fn compare_item_values(&self, a: Item, b: Item) -> Option<Ordering> {
        self.get_best_goal(a)
            .and_then(|a_g| self.get_best_goal(b).map(|b_g| a_g.cmp(&b_g)))
    }
}
