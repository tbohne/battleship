#[macro_use]
extern crate clap;
extern crate term_painter;
extern crate bincode;
#[macro_use]
extern crate serde_derive;

mod client;
mod model;
mod server;
mod view;

use bincode::serde::{serialize_into, deserialize_from, DeserializeError};
use clap::AppSettings;
use model::helper;
use model::types::SubField;
use std::net::{TcpStream};
use std::{time, thread};
use server::net::{types};
use std::str;
use server::net::types::MessageType;
use term_painter::ToStyle;
use term_painter::Color::*;

const BOARD_SIZE: u8 = 10;

fn main() {
    // initialize the CLI
    let battleship = clap_app!(battleship =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Two player battleship game")
        (@subcommand server =>
            (about: "Server instance for the game")
            (version: crate_version!())
            (author: crate_authors!())
            (@arg port: +required +takes_value "Connect to <port> on localhost")
            (@arg name: +required +takes_value "Name of player")
            (@arg size: -s --size +takes_value
                "set N as board dimension => N x N [not yet implemented]"
            )
            (@arg ships: --ships +takes_value "load ship configuration [not yet implemented]")
        )
        (@subcommand client =>
            (about: "Client instance for the game")
            (version: crate_version!())
            (author: crate_authors!())
            (@arg ip: +required +takes_value "Connect to address")
            (@arg port: +required +takes_value "Connect to port")
            (@arg name: +required +takes_value "Name of player")
        )
        (@subcommand single =>
            (about: "Play against the computer")
            (version: crate_version!())
            (author: crate_authors!())
            (@arg name: +required +takes_value "Name of player")
        )
    )
        .setting(AppSettings::SubcommandRequired)
        .get_matches();

    println!("Welcome to a round of 'battleship'");
    //model::start_round();

    match battleship.subcommand() {
        ("server", Some(server_args)) => {
            // required arguments
            let port = validate_port(server_args.value_of("port").unwrap());
            let name = String::from(server_args.value_of("name").unwrap());

            // optional arguments
            let mut size = BOARD_SIZE;
            if let Some(val) = server_args.value_of("size") {
                match val.parse::<u8>() {
                    Ok(val) => size = val,
                    Err(_) => size = helper::read_usize() as u8,
                }
            }

            if let Some(_) = server_args.value_of("ships") {
                println!("NOT IMPLEMENTED: loading custom ship configuration");
            }

            println!(
                "create server-player: '{}' -- connecting to port: {} -- {2}x{2} board",
                &name,
                port,
                size,
            );

            // create server
            let wait = thread::spawn(move || server::init(name, size));
            thread::sleep(time::Duration::from_millis(10));

            // create host player and connect to server
            //let host = TcpStream::connect((types::LOCALHOST, port)).unwrap();
            //thread::spawn(move || client::new(name, LOCALHOST, port));
            wait.join();
        },

        ("client", Some(client_args))   => {
            // for testing purpose
            const W: SubField = SubField::Water;
            const S: SubField = SubField::Ship;
            let testboard = vec![
                W,W,S,S,S,S,S,W,W,S,
                W,W,W,W,W,W,W,W,W,S,
                W,S,W,S,S,S,W,W,W,S,
                W,S,W,W,W,W,W,W,W,W,
                W,S,W,S,W,S,S,W,W,S,
                W,S,W,S,W,W,W,W,W,S,
                W,W,W,W,W,W,W,W,W,W,
                W,W,W,W,W,W,S,S,S,S,
                W,S,S,S,W,W,W,W,W,W,
                W,W,W,W,W,W,S,S,W,W,
            ];

            // required arguments
            let ip = client_args.value_of("ip").unwrap();
            // TODO: check for valid ip-address
            let port = validate_port(client_args.value_of("port").unwrap());
            let name = client_args.value_of("name").unwrap();

            println!(
                "create client-player: '{}' -- connecting to {}:{}",
                name,
                ip,
                port,
            );

            // create client instance and connect to server
            let mut client_connection = TcpStream::connect((ip, port)).unwrap();
            let client = client::new(name, ip, port);

            let mut host_name = "SERVER".to_string();
            loop {
                //let mut buffer = Vec::new();
                //let msg = client_connection.read_to_end(&mut buffer);
                //println!("{}", str::from_utf8(&buffer).unwrap());
                let recv: Result<types::MessageType, DeserializeError> =
                    deserialize_from(&mut client_connection, bincode::SizeLimit::Infinite);
                match recv {
                    Ok(received) => {
                        println!("RP: {:?}", received);
                        // process_message(received);
                        match received {
                            MessageType::Welcome(msg, host) => {
                                println!("{}", msg);
                                host_name = host;
                                // send Login data
                                serialize_into(
                                    &mut client_connection,
                                    &(types::MessageType::Login(name.to_string())),
                                    bincode::SizeLimit::Infinite
                                );
                            },
                            MessageType::Ping => {
                                // send Ping
                                serialize_into(
                                    &mut client_connection,
                                    &(types::MessageType::Ping),
                                    bincode::SizeLimit::Infinite
                                );
                            },
                            MessageType::Quit => {
                                println!("Server ended the connection.");
                                break;
                            },
                            MessageType::Board(b) => {
                                // TODO
                                model::print_boards(&b, &vec![SubField::Water; 100]);
                                // board = place(client, ...)
                            },
                            MessageType::RequestCoord => {
                                print!("It's your turn! ");
                                // send coordinate to shoot
                                let mut coord;
                                loop {
                                    println!("Please enter a valid coordinate: ");
                                    coord = ::helper::read_string();
                                    if ::model::valid_coordinate(&coord) {
                                        break;
                                    }
                                    print!("{}", Red.paint("Invalid coordinate! "));
                                }

                                server::net::send(&mut client_connection, MessageType::Shoot(coord));
                            }
                            MessageType::RequestBoard => {
                                // let client set all its ships
                                loop {
                                    // while not all ships set
                                    break;
                                }
                                // send board
                                server::net::send_board(&mut client_connection, &testboard);
                            }
                            MessageType::TurnHost => {
                                println!("Wait for {} to finish turn!", Yellow.paint(&host_name));
                            }
                            MessageType::Lost => {
                                println!("{}", Yellow.paint("You lost the game :("));
                            }
                            MessageType::Won => {
                                println!("{}", Yellow.paint("Congratulations, you won the game :)"));
                            }
                            MessageType::Unexpected => {
                                // resend expected packet
                            }
                            _ => {
                                println!("{}", Red.paint("Received unexpected packet"));
                            }
                        }
                    },
                    Err(_) => {
                        println!("Nothing to read - connection dropped...");
                        break;
                    },
                };


            }
        },

        ("single", Some(single_args)) => {
            let name = single_args.value_of("name").unwrap();

            println!(
                "create player: '{}'",
                name
            );

            // TODO: create game instance + AI
        },
        _ => {}, // Either no subcommand or one not tested for...
    }

    println!("");
}

/// Validate port
/// Only allow usage of ports 1024 up to 65535
fn validate_port(p: &str) -> u16 {
    let mut port = match p.parse::<u16>() {
        Ok(p) => p,
        Err(_) => {
            println!("Please enter a valid port (1024-65535): ");
            helper::read_usize() as u16
        },
    };

    loop {
        if port < 1024 {
            println!("Please enter a valid port (1024-65535): ");
            port = helper::read_usize() as u16;
        } else {
            break;
        }
    }
    port
}

fn process_message(msg: server::net::types::MessageType) {

}