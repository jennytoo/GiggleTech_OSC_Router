// GiggleTech.io
// OSC Router
// by Sideways

// External crates
use async_osc::{prelude::*, OscPacket, OscSocket, OscType, Result};
use async_std::{
    channel::{Receiver, Sender},
    stream::StreamExt,
    task::{self, JoinHandle},
};
use configparser::ini::Ini;
use lazy_static::lazy_static;
use std::{sync::Mutex, time::{Duration, Instant}};
use std::net::SocketAddr;

// Banner
fn banner_txt(){
    // https://fsymbols.com/generators/carty/
    println!("");
    println!("  ██████  ██  ██████   ██████  ██      ███████     ████████ ███████  ██████ ██   ██ ");
    println!(" ██       ██ ██       ██       ██      ██             ██    ██      ██      ██   ██ ");
    println!(" ██   ███ ██ ██   ███ ██   ███ ██      █████          ██    █████   ██      ███████ ");
    println!(" ██    ██ ██ ██    ██ ██    ██ ██      ██             ██    ██      ██      ██   ██ ");
    println!("  ██████  ██  ██████   ██████  ███████ ███████        ██    ███████  ██████ ██   ██ ");
    println!("");
    println!(" █▀█ █▀ █▀▀   █▀█ █▀█ █ █ ▀█▀ █▀▀ █▀█");
    println!(" █▄█ ▄█ █▄▄   █▀▄ █▄█ █▄█  █  ██▄ █▀▄");
                                                                                
}

// Configuation Loader
fn load_config() -> (
    String, // headpat_device_ip
    String, // headpat_device_port
    f32,    // min_speed_float
    f32,    // max_speed_float
    f32,    // speed_scale_float
    String, // port_rx
    String, // proximity_parameter_address
    String, // max_speed_parameter_address
    f32,    // Max Speed Low Limit
    ) {
    let mut config = Ini::new();

    match config.load("./config.ini") {
        Err(why) => panic!("{}", why),
        Ok(_) => {}
    }
    const MAX_SPEED_LOW_LIMIT_CONST: f32 = 0.05;

    let headpat_device_ip   = config.get("Setup", "device_ip").unwrap();
    let headpat_device_port = "8888".to_string();
    let min_speed           = config.get("Haptic_Config", "min_speed").unwrap();
    let min_speed_float     = min_speed.parse::<f32>().unwrap() / 100.0;
    let max_speed           = config.get("Haptic_Config", "max_speed").unwrap();
    let max_speed_float     = max_speed.parse::<f32>().unwrap() / 100.0; 
    let max_speed_low_limit = MAX_SPEED_LOW_LIMIT_CONST;
    let max_speed_float     = max_speed_float.max(max_speed_low_limit);
    let speed_scale         = config.get("Haptic_Config", "max_speed_scale").unwrap();
    let speed_scale_float   = speed_scale.parse::<f32>().unwrap() / 100.0;
    let port_rx             = config.get("Setup", "port_rx").unwrap();

    let proximity_parameter_address = config
        .get("Setup", "proximity_parameter")
        .unwrap_or_else(|| "/avatar/parameters/proximity_01".into());
    let max_speed_parameter_address = config
        .get("Setup", "max_speed_parameter")
        .unwrap_or_else(|| "/avatar/parameters/max_speed".into());

    println!("\n");
    banner_txt();
    println!("\n");
    println!(" Haptic Device: {}:{}", headpat_device_ip, headpat_device_port);
    println!(" Listening for OSC on port: {}", port_rx);
    println!("\n Vibration Configuration");
    println!(" Min Speed: {}%", min_speed);
    println!(" Max Speed: {:?}%", max_speed_float * 100.0);
    println!(" Scale Factor: {}%", speed_scale);
    println!("\nWaiting for pats...");

    (
        headpat_device_ip,
        headpat_device_port,
        min_speed_float,
        max_speed_float,
        speed_scale_float,
        port_rx,
        proximity_parameter_address,
        max_speed_parameter_address,
        max_speed_low_limit,
    )
}

// Make it easy to see prox when looking at router
fn proximity_graph(proximity_signal: f32) -> String {
    let num_dashes = (proximity_signal * 10.0) as usize;
    let graph = "-".repeat(num_dashes) + ">";

    graph
}

fn print_speed_limit(headpat_max_rx: f32) {
    let headpat_max_rx_print = (headpat_max_rx * 100.0).round() as i32;
    let max_meter = match headpat_max_rx_print {
        91..=i32::MAX => "!!! SO MUCH !!!",
        76..=90 => "!! ",
        51..=75 => "!  ",
        _ => "   ",
    };
    println!("Speed Limit: {}% {}", headpat_max_rx_print, max_meter);
}

// Pat Processor

const MOTOR_SPEED_SCALE: f32 = 0.66; // Overclock Here, OEM config 0.66 going higher than this value will reduce your vibrator motor life

fn process_pat(proximity_signal: f32, max_speed: f32, min_speed: f32, speed_scale: f32) -> i32 {
    let graph_str = proximity_graph(proximity_signal);
    let headpat_tx = (((max_speed - min_speed) * proximity_signal + min_speed) * MOTOR_SPEED_SCALE * speed_scale * 255.0).round() as i32;
    let proximity_signal = format!("{:.2}", proximity_signal);
    let max_speed = format!("{:.2}", max_speed);

    eprintln!("Prox: {:5} Motor Tx: {:3}  Max Speed: {:5} |{:11}|", proximity_signal, headpat_tx, max_speed, graph_str);
    
    headpat_tx
}


// REFACTOR BELOW ---------------




// Stop function
use tokio::select;
use async_std::channel::unbounded;

//use futures::future::select;
async fn my_async_function(stop_receiver: Receiver<()>) {
    println!("Async function started");
    loop {
        select! {
            _ = stop_receiver.recv() => break,
            _ = futures::future::pending() => {
                println!("Async function running"); // ----------------- WHEN NOT RX OSC SIGNAL BLINK THE LED to indicate osc router connection
                println!("boop");
            }
        }
    }
    println!("Async function stopped");
}


// Call stop function
async fn stop_async_task(stop_sender: Sender<()>, mut my_async_task: JoinHandle<()>) {
    //task::sleep(Duration::from_secs(5)).await;
    stop_sender.send(()).await.unwrap();
    my_async_task.await;
}


// Tx & Rx Socket Setup

fn create_socket_address(host: &str, port: &str) -> String {
    let address_parts = vec![host, port];
    address_parts.join(":")
}

async fn setup_rx_socket(port: std::string::String) -> Result<OscSocket> {
    let rx_socket_address = create_socket_address("127.0.0.1", &port.to_string());
    let rx_socket = OscSocket::bind(rx_socket_address).await?;
    Ok(rx_socket)
}

async fn setup_tx_socket(address: std::string::String) -> Result<OscSocket> {
    let tx_socket = OscSocket::bind("0.0.0.0:0").await?;
    tx_socket.connect(address).await?;
    Ok(tx_socket)
}

// OSC Address Setup
const TX_OSC_MOTOR_ADDRESS: &str = "/avatar/parameters/motor";
//const TX_OSC_LED_ADDRESS_2: &str = "/avatar/parameters/led";




// TimeOut 
/* 
lazy_static! {
    static ref LAST_SIGNAL_TIME: Mutex<Instant> = Mutex::new(Instant::now());
}
*/
lazy_static! {
    static ref LAST_SIGNAL_TIME: Mutex<Instant> = Mutex::new(Instant::now());
}


async fn stop_packet_timer(mut tx_socket: OscSocket) -> Result<()> {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        let elapsed_time = Instant::now().duration_since(*LAST_SIGNAL_TIME.lock().unwrap());

        if elapsed_time >= Duration::from_secs(5) {
            // Send stop packet
            println!("Pat Timeout...");
            tx_socket.send((TX_OSC_MOTOR_ADDRESS, (0i32,))).await?;

            let mut last_signal_time = LAST_SIGNAL_TIME.lock().unwrap();
            *last_signal_time = Instant::now();
        }
    }
}


#[async_std::main]
async fn main() -> Result<()> {
     
    // Import Config 
    let (headpat_device_ip,
        headpat_device_port,
        min_speed,
        mut max_speed,
        speed_scale,
        port_rx,
        proximity_parameter_address,
        max_speed_parameter_address,
        max_speed_low_limit,

    ) = load_config();



    let mut rx_socket = setup_rx_socket(port_rx).await?;

    let tx_socket_address = create_socket_address(&headpat_device_ip, &headpat_device_port);
    let tx_socket = setup_tx_socket(tx_socket_address.clone()).await?;
    let tx_socket_clone = setup_tx_socket(tx_socket_address).await?;

/*  Facny code that dosnt work
    let mut rx_socket = setup_rx_socket(port_rx).await?;
    let tx_socket_address = create_socket_address(&headpat_device_ip, &headpat_device_port);
    let (tx_socket, tx_socket_clone) = setup_tx_socket(tx_socket_address.clone()).await?;
*/
    //println!("The type of tx_socket_clone is: {}", std::any::type_name::<typeof(tx_socket_clone)>());


    task::spawn(stop_packet_timer(tx_socket_clone));
    
    // ---[ Stop Packet Timer ] ---
    //
    // Spawn a task to send stop packets when no signal is received for 5 seconds
/* 
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(1)).await;
            let elapsed_time = Instant::now().duration_since(*LAST_SIGNAL_TIME.lock().unwrap());
            
            if elapsed_time >= Duration::from_secs(5) {
                // Send stop packet
                println!("Pat Timeout...");
                tx_socket_clone.send((TX_OSC_MOTOR_ADDRESS, (0i32,))).await.ok();
                

                let mut last_signal_time = LAST_SIGNAL_TIME.lock().unwrap();

                *last_signal_time = Instant::now();            

            }
        }
    });
 */















    // Listen for OSC Packets
    while let Some(packet) = rx_socket.next().await {
        let (packet, _peer_addr) = packet?;
        ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////// See below for stop function
        let (stop_sender, stop_receiver) = unbounded::<()>();


        // Filter OSC Signals : Headpat Max & Headpat Prox 
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {

                let (address, osc_value) = message.as_tuple();

                let value = match osc_value.first().unwrap_or(&OscType::Nil).clone().float(){
                    Some(v) => v, 
                    None => continue,
                };

                if address == max_speed_parameter_address {
                    
                    print_speed_limit(value); 
                    max_speed = value;
                    if max_speed < max_speed_low_limit {
                        max_speed = max_speed_low_limit;
                    }
                }
                
                
                
                else if address == proximity_parameter_address  {
                    
                    // Update Last Signal Time for timeout clock
                    let mut last_signal_time = LAST_SIGNAL_TIME.lock().unwrap();
                    let elapsed_time = Instant::now().duration_since(*last_signal_time);
                    *last_signal_time = Instant::now();
                    
                    println!("{}", value);

                    
                    if value == 0.0 {
                        // Send 5 Stop Packets to Device - need to update so it sends stop packets until a new prox signal is made
                        println!("Stopping pats...");
                        
                        // Stop function

                        //let (stop_sender, stop_receiver) = unbounded::<()>();
                        let my_async_task = task::spawn(my_async_function(stop_receiver));

                        /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
                        //task::sleep(Duration::from_secs(0)).await;
                        //stop_sender.send(()).await.unwrap();
                        //my_async_task.await;


                        

                    
                        for _ in 0..5 {
                            println!("Send Stop...");
                            tx_socket
                                .send((TX_OSC_MOTOR_ADDRESS, (0i32,)))
                                .await?;
                        }
                    } else {
                        // Process Pat signal to send to Device   
                        let motor_speed_tx = process_pat(value, max_speed, min_speed, speed_scale);
                        
                        tx_socket
                            .send((TX_OSC_MOTOR_ADDRESS, (motor_speed_tx,)))
                            .await?;
                    }

                }
                else {
                    //eprintln!("Unknown Address") // Have a debug mode, print if debug mode
                }

            }
            
        }  
   
    }
    Ok(())
}
