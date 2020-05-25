mod items;
mod preference_list;

use colored::*;
use items::discretes::{Goal, Item};
use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use linefeed::{Interface, Prompter, ReadResult};
use preference_list::{Actor, GoalData};
use rand::seq::IteratorRandom;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use structopt::StructOpt;

fn main() -> io::Result<()> {
    let goal_hierarchy: Vec<GoalData> = vec![
        GoalData::RegularSatisfaction {
            goal: Goal::Eat,
            id: 0,
            time_required: 10,
            time: 0,
            units_required: 2,
            units: 0,
        },
        GoalData::Satisfaction {
            goal: Goal::Shelter,
            id: 1,
            units_required: 10,
            units: 0,
        },
        GoalData::RegularSatisfaction {
            goal: Goal::Rest,
            id: 2,
            time_required: 30,
            time: 0,
            units_required: 10,
            units: 0,
        },
        GoalData::Satisfaction {
            goal: Goal::Leisure,
            id: 3,
            units_required: 4,
            units: 1,
        },
    ];
    let opts: Cli = Cli::from_args();
    let mut trng = rand::thread_rng();
    let actors: HashMap<String, RefCell<Actor>> = (0..opts.actor_number)
        .map(|i: i32| {
            let mut a = Actor::new(
                format!("Actor#{}", i),
                goal_hierarchy.clone(),
                vec![
                    (Goal::Eat, vec![Item::FoodUnit]),
                    (Goal::Shelter, vec![Item::HouseUnit]),
                    (
                        Goal::Leisure,
                        vec![
                            Item::FoodUnit,
                            Item::HouseUnit,
                            *vec![Item::LeisureUnit1, Item::LeisureUnit2]
                                .iter()
                                .choose(&mut trng)
                                .unwrap(),
                        ],
                    ),
                ],
            );
            if let Some(ri) = vec![
                vec![Item::FoodUnit, Item::FoodUnit, Item::FoodUnit],
                vec![Item::HouseUnit, Item::FoodUnit],
                vec![Item::LeisureUnit1, Item::LeisureUnit2],
                vec![
                    Item::FoodUnit,
                    Item::FoodUnit,
                    Item::FoodUnit,
                    Item::LeisureUnit2,
                ],
            ]
            .iter()
            .choose(&mut trng)
            {
                a.inventory.extend(ri.iter());
            }
            (a.name.clone(), RefCell::new(a))
        })
        .collect();

    println!("Welcome to the microeconomic actor prototype interactive interface.");
    println!("Enter \"help\" for a list of commands.");
    println!("Press Ctrl-D or enter \"quit\" to exit.");
    println!("");

    let reader = Interface::new("microeconomics")?;
    reader.set_completer(Arc::new(InterfaceCompleter(
        actors.keys().into_iter().map(|x| x.clone()).collect(),
    )));
    reader.set_prompt(&"interaction> ".bold().blue().to_string())?;

    while let ReadResult::Input(input) = reader.read_line()? {
        if !input.trim().is_empty() {
            reader.add_history_unique(input.clone());
        }
        let cmd: Vec<&str> = input.trim().split_ascii_whitespace().collect();
        match &*cmd {
            ["help"] => {
                println!("actor interface commands:");
                println!();
                for &(cmd, help) in INT_COMMANDS {
                    println!("  {:20} - {}", cmd.green(), help);
                }
                println!();
            }
            ["get-actor", property, actorid] => {
                if let Some(actor) = actors.get(&actorid.to_string()) {
                    let actor = actor.borrow();
                    match *property {
                        "preference-list" => {
                            println!("ordinal hierarchy of items for {}:", actorid.yellow());
                            println!("");
                            println!(
                                "{:20} | {:20} | {:20}",
                                "Item".bold(),
                                "Highest-Valued Goal".bold(),
                                "# Goals".bold()
                            );
                            let twenty = "-".to_string().repeat(20);
                            println!("{}-+-{}-+-{}", twenty, twenty, twenty);
                            for (item, bh) in actor.preference_list.iter() {
                                println!(
                                    "{:20} | {:20} | {:20}",
                                    format!("{:?}", item).green(),
                                    if let Some(g) = bh.peek() {
                                        format!("{:?}", g.goal).blue()
                                    } else {
                                        "N/A".to_string().blue()
                                    },
                                    format!("{:?}", bh.capacity())
                                );
                            }
                            println!("");
                        }
                        "state" => {
                            println!("general AI state for {}:", actorid.yellow());
                            println!("");
                            println!("- ACTOR STATE");
                            println!("  {}", format!("{:?}", actor.state).yellow());
                            println!("");
                            println!("- CURRENT GOALS IN PLAY");
                            println!("  {:10} | {:10}", "Goal".bold(), "Index".bold());
                            println!("  {:-^1$}", "+", 23);
                            let mut sorted_goals: Vec<_> = actor
                                .current_goals
                                .iter()
                                .map(|x| (x.goal, actor.goal_hierarchy.get(&x.goal).unwrap()))
                                .collect();
                            sorted_goals.sort_by_key(|f| f.1);
                            for (goal, index) in sorted_goals {
                                println!(
                                    "  {:10} | {:10}",
                                    format!("{:?}", goal).blue(),
                                    format!("{:?}", index)
                                );
                            }
                            println!("");
                            println!("- INVENTORY");
                            println!(
                                "  {:20} | {:20} | {:20}",
                                "Item".bold(),
                                "Highest-Valued Goal".bold(),
                                "# Goals".bold()
                            );
                            let twenty = "-".to_string().repeat(20);
                            println!("  {}-+-{}-+-{}", twenty, twenty, twenty);
                            for item in actor.inventory.iter() {
                                let bh = actor.preference_list.get(&item);
                                println!(
                                    "  {:20} | {:20} | {:20}",
                                    format!("{:?}", item).green(),
                                    if let Some(g) = bh.and_then(|x| x.peek()) {
                                        format!("{:?}", g.goal).blue()
                                    } else {
                                        "N/A".to_string().blue()
                                    },
                                    format!("{:?}", bh.map(|x| x.capacity()).unwrap_or(0))
                                );
                            }
                            println!("");
                        }
                        "goal-registry" => {
                            println!("goal details for {}:", actorid.yellow());
                            let mut registry: Vec<(&Goal, &GoalData)> =
                                actor.goal_registry.iter().collect();
                            registry.sort_by_key(|(g, _)| actor.goal_hierarchy.get(g).unwrap());
                            for (goal, goal_data) in registry {
                                println!("");
                                println!("- {}", format!("{:?}", goal).blue());
                                println!("  {:?}", goal_data);
                            }
                            println!("");
                        }
                        "goal-hierarchy" => {
                            println!("ordinal hierarchy of values for {}:", actorid.yellow());
                            println!("");
                            println!("{:10} | {:10}", "Goal".bold(), "Index".bold());
                            println!("{:-^1$}", "+", 23);
                            let mut sorted_goals: Vec<_> = actor.goal_hierarchy.iter().collect();
                            sorted_goals.sort_by_key(|f| f.1);
                            for (goal, index) in sorted_goals {
                                println!(
                                    "{:10} | {:10}",
                                    format!("{:?}", goal).blue(),
                                    format!("{:?}", index)
                                );
                            }
                            println!("");
                        }
                        x => println!("{} {}", "unknown subcommand:".red(), x),
                    }
                } else {
                    println!("cannot find actor: {}", actorid);
                }
            }
            ["compare-item-values", actor, item1, item2] => {
                use Item::*;
                let i1 = match *item1 {
                    "FoodUnit" => FoodUnit,
                    "HouseUnit" => HouseUnit,
                    "LeisureUnit1" => LeisureUnit1,
                    "LeisureUnit2" => LeisureUnit2,
                    _ => panic!("unrecognized item"),
                };
                let i2 = match *item2 {
                    "FoodUnit" => FoodUnit,
                    "HouseUnit" => HouseUnit,
                    "LeisureUnit1" => LeisureUnit1,
                    "LeisureUnit2" => LeisureUnit2,
                    _ => panic!("unrecognized item"),
                };
                match actors
                    .get(&actor.to_string())
                    .unwrap()
                    .borrow()
                    .compare_item_values(i1, i2)
                {
                    Some(Ordering::Equal) => println!("These items are valued the same!"),
                    Some(Ordering::Less) => println!(
                        "{} is valued less than {}",
                        item1.to_string().green(),
                        item2.to_string().green()
                    ),
                    Some(Ordering::Greater) => println!(
                        "{} is valued more than {}",
                        item1.to_string().green(),
                        item2.to_string().green()
                    ),
                    None => println!("{}", "actor does not recognize one of these items".red()),
                }
            }
            ["tick"] => {
                for (_, actor) in actors.iter() {
                    actor.borrow_mut().tick(actors.values().collect());
                }
            }
            ["give-item", actor, item] => {
                use Item::*;
                if let Some(actor) = actors.get(&actor.to_string()) {
                    let mut actor = actor.borrow_mut();
                    actor.add_item(match *item {
                        "FoodUnit" => FoodUnit,
                        "HouseUnit" => HouseUnit,
                        "LeisureUnit1" => LeisureUnit1,
                        "LeisureUnit2" => LeisureUnit2,
                        _ => panic!("unrecognized item"),
                    });
                } else {
                    println!("{}", "unrecognized actor".red())
                }
            }
            ["quit"] => return Ok(()),
            _ => println!("{} {}", "unrecognized command".red(), cmd.join(" ")),
        }
    }

    println!("Exiting...");
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "microeconomics",
    about = "A simple Austrian microeconomic actor prototype."
)]
struct Cli {
    /// Number of actors to use
    actor_number: i32,
}

static INT_COMMANDS: &[(&str, &str)] = &[
    ("help", "You're looking at it"),
    (
        "get-actor",
        "Get a prop (preference-list, goal-hierarchy, goal-registry, state)",
    ),
    ("tick", "Tick time forward and run simulation on its own"),
    ("give-item", "Add an item to an actor's inventory"),
    (
        "compare-item-values",
        "Have an actor compare two item's values",
    ),
    ("quit", "Quit the interactive interface"),
];

struct InterfaceCompleter(Vec<String>);

impl<Term: Terminal> Completer<Term> for InterfaceCompleter {
    fn complete(
        &self,
        word: &str,
        prompter: &Prompter<Term>,
        start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let line = prompter.buffer();

        let mut words = line[..start].split_whitespace();

        match words.next() {
            // Complete command name
            None => {
                let mut compls = Vec::new();

                for &(cmd, _) in INT_COMMANDS {
                    if cmd.starts_with(word) {
                        compls.push(Completion::simple(cmd.to_owned()));
                    }
                }

                Some(compls)
            }
            // Complete command parameters
            Some("get-actor") => {
                let wc = words.count();
                if wc == 0 {
                    let mut res = Vec::new();

                    for subcmd in vec![
                        "preference-list",
                        "goal-hierarchy",
                        "goal-registry",
                        "state",
                    ] {
                        if subcmd.starts_with(word) {
                            res.push(Completion::simple(subcmd.to_owned()));
                        }
                    }

                    Some(res)
                } else if wc == 1 {
                    let mut res = Vec::new();

                    for actor_name in self.0.iter() {
                        if actor_name.starts_with(word) {
                            res.push(Completion::simple(actor_name.to_owned()));
                        }
                    }

                    Some(res)
                } else {
                    None
                }
            }
            Some("compare-item-values") => {
                let wc = words.count();
                if wc == 0 {
                    let mut res = Vec::new();

                    for actor_name in self.0.iter() {
                        if actor_name.starts_with(word) {
                            res.push(Completion::simple(actor_name.to_owned()));
                        }
                    }

                    Some(res)
                } else if wc == 1 || wc == 2 {
                    let mut res = Vec::new();

                    for item in vec!["FoodUnit", "HouseUnit", "LeisureUnit1", "LeisureUnit2"] {
                        if item.starts_with(word) {
                            res.push(Completion::simple(item.to_owned()));
                        }
                    }

                    Some(res)
                } else {
                    None
                }
            }
            Some("give-item") => {
                let wc = words.count();
                if wc == 0 {
                    let mut res = Vec::new();

                    for actor_name in self.0.iter() {
                        if actor_name.starts_with(word) {
                            res.push(Completion::simple(actor_name.to_owned()));
                        }
                    }

                    Some(res)
                } else if wc == 1 {
                    let mut res = Vec::new();

                    for item in vec!["FoodUnit", "HouseUnit", "LeisureUnit1", "LeisureUnit2"] {
                        if item.starts_with(word) {
                            res.push(Completion::simple(item.to_owned()));
                        }
                    }

                    Some(res)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
