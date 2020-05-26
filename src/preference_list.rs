use crate::items::discretes::Goal;
use crate::items::discretes::Item;
use colored::*;
use std::cell::RefCell;
use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::rc::Rc;

/// Contains all of the metadata required to satisfy a goal properly. This data
/// is stored only in the preference list of the actor and the recurrance list
/// of the actor, since the preference list is the data structure that is
/// actually used when satisfying goals, and the recurrance list is the only
/// place where the metadata about recurrance time intervals matter. I could
/// have designed separate data structures for those two peices of information,
/// but that would've been unweildy in my opinion.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GoalData {
    /// A goal that either occurs at random times or only once.
    Satisfaction {
        /// The goal to be satisfied
        goal: Goal,
        /// Amount of acceptable units needed to satisfy this goal
        units_required: i32,
        /// Current units diverted to this goal
        units: i32,
        /// Unique id
        id: i32,
    },
    /// A regularly recurring goal.
    RegularSatisfaction {
        /// The goal to be satisfied
        goal: Goal,
        /// Time required for this goal to reoccur
        time_required: i32,
        /// Time since this goal was dismissed
        time: i32,
        /// Amount of acceptable units needed to satisfy this goal
        units_required: i32,
        /// Current units diverted to this goal
        units: i32,
        /// Unique id
        id: i32,
    },
}

impl GoalData {
    /// Get the goal this metadata might satisfy
    pub fn get_goal(&self) -> Goal {
        match self {
            &GoalData::Satisfaction { goal, .. } | &GoalData::RegularSatisfaction { goal, .. } => {
                goal
            }
        }
    }

    /// Check if this goal should be in the recurrance list
    pub fn is_recurring(&self) -> bool {
        match self {
            &GoalData::Satisfaction { .. } => false,
            _ => true,
        }
    }
}

/// This is necessary to take advantage of the automatic sorting abilities of
/// the BinaryHeap that we use in the preference list. This only exists because
/// of that, there's nothing special about this otherwise.
pub struct GoalWrapper {
    /// Closure that encloses a reference-counted pointer to the goal hierarchy
    /// of the containing actor so it can do comparasons.
    comparator: Box<dyn Fn(&Goal, &Goal) -> Ordering>,
    /// The actual interesting data that we want the BinaryHeap to sort
    pub goal: Goal,
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

/// A map of the item that must be valued or used to the max-heap containing the
/// goals that can be satisfied with the item. Since the most highly-valued goal
/// is the one that will always be referenced for both use and valuing, those
/// operations need only ever deal with the root of the heap, making this very
/// performant.
pub type PreferenceList = HashMap<Item, BinaryHeap<Rc<GoalWrapper>>>;

/// Individual acting, valuing, satisfying Austrian microeconomic actor
pub struct Actor {
    /// Name for printouts
    pub name: String,
    /// Registry of data for goals (to avoid needing interior mutability, etc)
    pub goal_registry: HashMap<Goal, GoalData>,
    /// Absolute list of goals to use for actions
    pub current_goals: BinaryHeap<Rc<GoalWrapper>>,
    /// Mapping of items to their goals
    pub preference_list: PreferenceList,
    /// Mapping of goals to the items that can satisfy them
    pub satisfactions: HashMap<Goal, Vec<Item>>,
    // TODO: Make sure that goal heirarchy is strictly ordinal.
    /// How much goals are valued. This could easily be stored as a list, and in
    /// fact is constructed from one, but is more performant for our purposes as
    /// a map from a goal to how much it is valued.
    pub goal_hierarchy: HashMap<Goal, usize>,
    /// Items the actor has already (for trade or use)
    pub inventory: Vec<Item>,
    /// Actor internal AI state
    pub state: ActorState,
}

/// The state the actor is in for one tick (reset at the start of every tick)
#[derive(PartialEq, Debug)]
pub enum ActorState {
    /// Needs a goal (previous goals satisfied)
    SearchingForGoal,
    /// Needs to trade to get an item, needs to do that on next tick
    WillingToTrade(i32),
    /// Found an actor to try bidding with, begin bidding on next tick
    FoundTradePartner(usize),
    /// Current bid, this is used for actor on the initiating side
    Bidding(usize, i32, i32),
    /// A state for the actor waiting on the other side of a bid, so that it doesn't consume items needed for the trade.
    // TODO: Store value of other's bid in such a way that another can jump in and bid higher
    BidRecipiant(usize, Option<Item>, Option<(usize, Item)>),
}

impl Actor {
    /// Construct a new actor. Does some housekeeping to make construction easier.
    ///
    /// # Arguments
    ///
    /// * `name` - actor's name, for printout results
    /// * `hierarchy` - list of actor's valued ends as `GoalData` so that they can also be added to other places.
    ///
    pub fn new(
        name: String,
        hierarchy: Vec<GoalData>,
        satisfactions: Vec<(Goal, Vec<Item>)>,
    ) -> Self {
        let mut this = Actor {
            name: name,
            current_goals: BinaryHeap::new(),
            goal_registry: HashMap::new(),
            preference_list: HashMap::new(),
            satisfactions: satisfactions.into_iter().collect(),
            goal_hierarchy: HashMap::new(),
            inventory: vec![],
            state: ActorState::SearchingForGoal,
        };
        for (i, goal) in hierarchy.into_iter().enumerate() {
            this.add_new_goal(goal, i);
        }
        this
    }

    /// This function runs the actor's simplified-praxeology choice-AI for one
    /// tick, where ticks are the time unit of the recurring goals, and consist
    /// of one action (see README).
    ///
    /// # Arguments
    ///
    /// * `other_actors` - list of the other actors available to trade with
    ///
    pub fn tick(&mut self, other_actors: &Vec<RefCell<Actor>>) {
        let mut reintroduce_goals = vec![];
        for (goal, goal_data) in self.goal_registry.iter_mut() {
            if let GoalData::RegularSatisfaction {
                time,
                time_required,
                ..
            } = goal_data
            {
                *time += 1;
                if *time >= *time_required {
                    *time = 0;
                    reintroduce_goals.push(goal.clone());
                }
            }
        }
        for goal in reintroduce_goals {
            self.add_goal(goal);
        }

        // Get the highest-valued goal of the ones that are in play
        let goal = self.current_goals.peek().map(|x| x.goal);
        println!(
            "{} selects {} as a goal",
            self.name.yellow(),
            format!("{:?}", goal).blue()
        );

        // If it exists...
        if let Some(goal) = goal {
            match self.state {
                ActorState::SearchingForGoal => {
                    // ...try to find all of the items that *might* be able to satisfy this goal
                    let possibilities = self.find_item_for_goal(goal);
                    println!(
                        "{} finds {} items for use on goal {}",
                        self.name.yellow(),
                        possibilities.len(),
                        format!("{:?}", goal).blue()
                    );
                    if possibilities.len() >= 1 {
                        // We ended up finding a viable item, so use it

                        // TODO: Add time-preference so that agents will wait if an item
                        // has a higher-valued goal, so they can use it for that goal,
                        // and decide to trade instead for their current goal (if the
                        // situation isn't too dire)
                        self.use_item_for_goal(*possibilities.last().unwrap(), goal);
                        self.state = ActorState::SearchingForGoal;
                    } else if possibilities.len() == 0 {
                        // We need an item
                        if self.inventory.len() > 0 {
                            println!("{} is now willing to trade", self.name.yellow(),);
                            self.state = ActorState::WillingToTrade(-1);
                        } else {
                            self.state = ActorState::SearchingForGoal;
                        }
                    }
                }
                ActorState::WillingToTrade(idx) => {
                    // Find trade partner
                    if let Some(first_idx) =
                        self.find_next_actor_for_trade(goal, &other_actors, idx)
                    {
                        self.state = ActorState::FoundTradePartner(first_idx);
                        let mut oa = other_actors[first_idx].borrow_mut();
                        oa.state = ActorState::BidRecipiant(
                            self.name
                                .split("#")
                                .nth(1)
                                .and_then(|x| x.parse::<usize>().ok())
                                .unwrap(),
                            None,
                            None,
                        );
                    } else {
                        self.state = ActorState::SearchingForGoal;
                    }
                }
                ActorState::FoundTradePartner(idx) => {
                    // Check if transaction is viable at all
                    let mut other_actor = other_actors[idx].borrow_mut();
                    let actors_items =
                        other_actor.has_item_of(self.satisfactions.get(&goal).unwrap());
                    let (item1, item2) = (
                        *self.inventory.last().unwrap(),
                        *actors_items.first().unwrap(),
                    );
                    println!(
                        "{}/{}: first tentative proposal bid: will give {} for {}",
                        self.name.yellow(),
                        other_actor.name.yellow(),
                        format!("{:?}", item1).green(),
                        format!("{:?}", item2.1).green(),
                    );
                    if self.compare_item_values(item1, item2.1) != Ordering::Less {
                        println!("This trade will never work");
                        other_actor.state = ActorState::SearchingForGoal;
                        self.state = ActorState::WillingToTrade(idx as i32);
                    } else {
                        // prepare to bid
                        println!(
                            "{} prepares to start bidding in earnest",
                            self.name.yellow()
                        );
                        self.state = ActorState::Bidding(
                            idx,
                            self.inventory.len() as i32,
                            item2.0 as i32 - 1,
                        );
                        match other_actor.state {
                            ActorState::BidRecipiant(idx, ..) => {
                                other_actor.state =
                                    ActorState::BidRecipiant(idx, Some(item1), Some(item2));
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                ActorState::Bidding(idx, prev_item1, prev_item2) => {
                    let mut other_actor = other_actors[idx].borrow_mut();
                    let actors_items =
                        other_actor.has_item_of(self.satisfactions.get(&goal).unwrap());
                    // if there's no more items for this actor, find another to
                    // trade with
                    if actors_items.last().unwrap().0 as i32 <= prev_item2 || prev_item1 <= 0 {
                        println!(
                            "{}/{}: No more items to trade, going to next actor.",
                            self.name.yellow(),
                            other_actor.name.yellow()
                        );
                        other_actor.state = ActorState::SearchingForGoal;
                        self.state = ActorState::WillingToTrade(idx as i32);
                    } else {
                        // get actor's next bid based on last bid
                        let (item1, item2) = (
                            self.inventory[(prev_item1 - 1) as usize],
                            actors_items[actors_items
                                .iter()
                                .position(|(i, _)| *i as i32 == prev_item2 + 1)
                                .unwrap()],
                        );
                        println!(
                            "{}/{} makes bid: will give {} for {}",
                            self.name.yellow(),
                            other_actor.name.yellow(),
                            format!("{:?}", item1).green(),
                            format!("{:?}", item2.1).green(),
                        );
                        let other =
                            other_actor.compare_item_values(item1, item2.1) != Ordering::Less;
                        let me = self.compare_item_values(item1, item2.1) != Ordering::Greater;
                        let bid_accepted = other && me;
                        if bid_accepted {
                            println!("Inventories before:");
                            println!(
                                "- {}: {}",
                                self.name.yellow(),
                                format!("{:?}", self.inventory).green()
                            );
                            println!(
                                "- {}: {}",
                                other_actor.name.yellow(),
                                format!("{:?}", other_actor.inventory).green()
                            );

                            // add other's item to inventory, remove it from theirs
                            self.add_item(item2.1);
                            self.inventory.remove((prev_item1 - 1) as usize);
                            // remove my item from my inventory, add it to theirs
                            other_actor.add_item(item1);
                            other_actor.inventory.remove(item2.0);

                            println!("Inventories after:");
                            println!(
                                "- {}: {}",
                                self.name.yellow(),
                                format!("{:?}", self.inventory).green()
                            );
                            println!(
                                "- {}: {}",
                                other_actor.name.yellow(),
                                format!("{:?}", other_actor.inventory).green()
                            );

                            self.state = ActorState::SearchingForGoal;
                            other_actor.state = ActorState::SearchingForGoal;
                            println!(
                                "{}/{}: {}",
                                self.name.yellow(),
                                other_actor.name.yellow(),
                                "Trade complete".green()
                            );
                        } else {
                            println!(
                                "{}/{}: bid rejected by {}",
                                self.name.yellow(),
                                other_actor.name.yellow(),
                                if !other && me {
                                    other_actor.name.yellow()
                                } else {
                                    self.name.yellow()
                                }
                            );
                            println!(
                                "{}/{}: bid is proceeding up",
                                self.name.yellow(),
                                other_actor.name.yellow()
                            );
                            self.state = ActorState::Bidding(idx, prev_item1 - 1, item2.0 as i32);
                            match other_actor.state {
                                ActorState::BidRecipiant(idx, ..) => {
                                    other_actor.state =
                                        ActorState::BidRecipiant(idx, Some(item1), Some(item2));
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
                ActorState::BidRecipiant(_idx, _i1, _i2) => {
                    println!("{} is waiting in bid", self.name.yellow());
                }
            }
        } else {
            println!("{} does not pursue any goals", self.name.yellow());
        }
        println!("");
    }

    fn find_next_actor_for_trade(
        &self,
        goal: Goal,
        other_actors: &Vec<RefCell<Actor>>,
        idx: i32,
    ) -> Option<usize> {
        for (idx, actor) in other_actors.iter().skip((idx + 1) as usize).enumerate() {
            if format!("Actor#{}", idx) == self.name {
                continue;
            }
            if let Ok(actor) = actor.try_borrow() {
                match actor.state {
                    ActorState::BidRecipiant(..) | ActorState::Bidding(..) => {
                        println!(
                            "{}: {} is already occupied trading with another actor, skipping",
                            self.name.yellow(),
                            format!("Actor#{}", idx).yellow()
                        );
                    }
                    _ => {
                        let actors_items =
                            actor.has_item_of(self.satisfactions.get(&goal).unwrap());
                        if actors_items.len() > 0 {
                            println!(
                                "{} finds trade partner {} with {} items it is interested in",
                                self.name.yellow(),
                                format!("Actor#{}", idx).yellow(),
                                actors_items.len()
                            );
                            return Some(idx);
                        }
                    }
                }
            } else {
                /*println!(
                    "{}: {} is already occupied trading with another actor, skipping (EO1)",
                    self.name.yellow(),
                    format!("Actor#{}", idx).yellow()
                );*/
            }
        }
        None
    }

    pub fn has_item_of(&self, items: &Vec<Item>) -> Vec<(usize, Item)> {
        self.inventory
            .iter()
            .enumerate()
            .filter(|(_, x)| items.contains(x))
            .map(|(i, x)| (i, x.clone()))
            .collect()
    }

    /// Adds item to inventory in a sorted manner
    pub fn add_item(&mut self, item: Item) {
        let loc = self
            .inventory
            .binary_search_by(|probe| self.compare_item_values(*probe, item))
            .unwrap_or_else(|e| e);
        self.inventory.insert(loc, item);
    }

    /// Finds any items in the inventory that might satisfy a goal.
    ///
    /// # Arguments
    ///
    /// * `goal` - goal to satisfy
    ///
    /// # Notes
    ///
    ///  If it finds an item whose highest-valued goal is that goal, it returns
    ///  a list of just that. Also, the list is sorted greatest-valued item to
    ///  least using an insertion sort (best I can do without adding a binheap
    ///  wrapper).
    fn find_item_for_goal(&self, goal: Goal) -> Vec<Item> {
        let mut possibilities = vec![];
        let opts = self.satisfactions.get(&goal).unwrap();
        for item in self.inventory.iter() {
            if opts.contains(&item) {
                // If we have an item whose best use is for this goal...
                if self
                    .preference_list
                    .get(&item)
                    .unwrap()
                    .peek()
                    .unwrap()
                    .goal
                    == goal
                {
                    // ...jackpot, use it!
                    possibilities.push(item.clone());
                    break;
                } else {
                    // ...otherwise, We want to use the least-valued
                    // item that can satisfy our need, so insert
                    // sortedly into vector (slow-ass, but I'm lazy and
                    // don't want to make another binary heap wrapper
                    // (argh!))
                    possibilities.push(item.clone());
                }
            }
        }
        possibilities
    }

    /// Adds a *new* goal (not already in registry) to all of the BinaryHeaps
    /// for all of the items that can satisfy it (sorted).
    ///
    /// # Arguments
    ///
    /// * `goal` - `GoalData` of what's to be added
    /// * `location` - the location for it to be inserted into the hierarchy of ends/values
    ///
    pub fn add_new_goal(&mut self, goal: GoalData, location: usize) {
        self.add_goal(goal.get_goal());
        self.goal_hierarchy.insert(goal.get_goal(), location);
        self.goal_registry.insert(goal.get_goal(), goal);
    }

    /// Adds a goal (already in registry and hierarchy) to all of the
    /// BinaryHeaps for all of the items that can satisfy it (sorted).
    ///
    /// # Arguments
    ///
    /// * `goal` - `Goal` of what's to be added, acts as ID into registry to get `GoalData`.
    /// * `location` - the location for it to be inserted into the hierarchy of ends/values
    ///
    pub fn add_goal(&mut self, goal: Goal) {
        println!(
            "{} reintroduces {}",
            self.name.yellow(),
            format!("{:?}", goal).blue()
        );
        let gh = self.goal_hierarchy.clone();
        let ordered_goal = Rc::new(GoalWrapper {
            comparator: Box::new(move |x: &Goal, y: &Goal| {
                let xval = gh.get(&x);
                let yval = gh.get(&y);
                xval.and_then(|x| yval.map(|y| x.cmp(y)))
                    .unwrap_or(Ordering::Equal)
            }),
            goal: goal,
        });
        if let Some(effected_entries) = self.satisfactions.get(&goal) {
            for item in effected_entries.iter() {
                self.preference_list
                    .entry(*item)
                    .or_insert(BinaryHeap::new())
                    .push(ordered_goal.clone());
            }
        }
        self.current_goals.push(ordered_goal);
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
                        self.preference_list.get(&item).map(
                            |goals: &BinaryHeap<Rc<GoalWrapper>>| {
                                for og in goals.into_iter() {
                                    if og.goal != actual_goal {
                                        new.push(og.clone());
                                    }
                                }
                            },
                        );
                        *self.preference_list.get_mut(&item).unwrap() = new;
                    }
                }
            }
        }

        let mut new = BinaryHeap::new();
        for og in self.current_goals.iter() {
            if og.goal != actual_goal {
                new.push(og.clone());
            }
        }
        self.current_goals = new;

        self.goal_registry.remove(&actual_goal);
        self.goal_hierarchy.remove(&actual_goal);
    }

    /// Uses an item to satisfy the goal selected
    ///
    /// # Arguments
    ///
    /// * `item` - `Item` to use
    /// * `goal` - `Goal` to satisfy
    ///
    /// # Notes
    ///
    /// Doesn't update recurring goals. See `tick`.
    ///
    pub fn use_item_for_goal(&mut self, item: Item, goal: Goal) {
        if let Some(idx) = self.inventory.iter().position(|&r| r == item) {
            self.inventory.remove(idx);
            let mut should_remove = false;
            {
                let highest_valued_goal: &mut GoalData = self.goal_registry.get_mut(&goal).unwrap();
                println!(
                    "{} uses item {} for goal {}",
                    self.name.yellow(),
                    format!("{:?}", item).green(),
                    format!("{:?}", goal).blue()
                );
                match highest_valued_goal {
                    GoalData::Satisfaction {
                        units_required,
                        units,
                        ..
                    } => {
                        *units += 1;
                        if *units >= *units_required {
                            should_remove = true;
                        }
                    }
                    GoalData::RegularSatisfaction {
                        units_required,
                        units,
                        ..
                    } => {
                        *units += 1;
                        if *units >= *units_required {
                            should_remove = true;
                        }
                    }
                }
            }
            if should_remove {
                self.remove_goal(goal);
            }
        } else {
            println!(
                "{} does not have item {} for goal {} in inventory",
                self.name.yellow(),
                format!("{:?}", item).green(),
                format!("{:?}", goal).blue()
            );
        }
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
            .map(|og| og.goal)
    }

    /// Compare two items to see which is more valuable based on the goals it can satisfy
    ///
    /// # Arguments
    ///
    /// * `a` - first item
    /// * `b` - second item
    ///
    pub fn compare_item_values(&self, a: Item, b: Item) -> Ordering {
        let gh = self.goal_hierarchy.clone();
        let a_val = self.get_best_goal(a).and_then(|a_g| gh.get(&a_g));
        let b_val = self.get_best_goal(b).and_then(|b_g| gh.get(&b_g));
        if a_val.is_none() && b_val.is_some() {
            Ordering::Less
        } else if a_val.is_some() && b_val.is_none() {
            Ordering::Greater
        } else if a_val.is_none() && b_val.is_none() {
            Ordering::Equal
        } else {
            b_val.unwrap().cmp(a_val.unwrap())
        }
    }
}
