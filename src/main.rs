mod items;
mod preference_list;

use colored::*;
use items::discretes::{Goal, Item};
use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use linefeed::{Interface, Prompter, ReadResult};
use preference_list::{Actor, GoalData};
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use structopt::StructOpt;

fn main() -> io::Result<()> {
    let GOAL_HIREARCHY: Vec<GoalData> = vec![
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
    let mut actors: HashMap<String, Actor> = (0..opts.actor_number)
        .map(|i: i32| {
            let mut a = Actor::new(format!("Actor#{}", i), GOAL_HIREARCHY.clone());
            a.add_satisfaction_entry(Goal::Eat, Item::FoodUnit);
            a.add_satisfaction_entry(Goal::Shelter, Item::HouseUnit);
            a.add_satisfaction_entry(
                Goal::Leisure,
                *vec![Item::LeisureUnit1, Item::LeisureUnit2]
                    .iter()
                    .choose(&mut trng)
                    .unwrap(),
            );
            (a.name.clone(), a)
        })
        .collect();

    println!("Welcome to the microeconomic actor prototype interactive interface.");
    println!("Enter \"help\" for a list of commands.");
    println!("Press Ctrl-D or enter \"quit\" to exit.");
    println!("");

    let mut reader = Interface::new("microeconomics")?;
    reader.set_completer(Arc::new(InterfaceCompleter));
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
            ["get-actor", actorid, property] => {
                if let Some(actor) = actors.get(&actorid.to_string()) {
                    match *property {
                        "preference-list" => {}
                        "goal-hierarchy" => {
                            println!("ordinal hierarchy of values for {}:", actorid);
                            println!("");
                            println!("{:10} | {:10}", "Goal".bold(), "Index".bold());
                            println!("{:-^1$}", "+", 23);
                            let mut sorted_goals: Vec<_> = actor.goal_hierarchy.0.iter().collect();
                            sorted_goals.sort_by_key(|f| f.1);
                            for (goal, index) in sorted_goals {
                                println!(
                                    "{:10} | {:10}",
                                    format!("{:?}", goal),
                                    format!("{:?}", index)
                                );
                            }
                            println!("");
                        }
                        x => println!("unknown subcommand: {}", x),
                    }
                } else {
                    println!("cannot find actor: {}", actorid);
                }
            }
            ["quit"] => return Ok(()),
            _ => println!("unrecognized command: {}", cmd.join(" ")),
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
        "Get an actor property (preference-list, goal-hierarchy)",
    ),
    ("tick", "Tick time forward and run simulation on its own"),
    ("add-goal", "Add a goal to an actor"),
    ("remove-goal", "Remove a goal from an actor"),
    ("use-item", "Have actor use item"),
    ("add-satisfaction", "Add item that can satisfy actor's goal"),
    (
        "compare-item-values",
        "Have an actor compare two item's values",
    ),
    ("quit", "Quit the interactive interface"),
];

struct InterfaceCompleter;

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
            Some("get") | Some("set") => {
                if words.count() == 0 {
                    let mut res = Vec::new();

                    for (name, _) in prompter.variables() {
                        if name.starts_with(word) {
                            res.push(Completion::simple(name.to_owned()));
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
