use std::{thread, time};

use chain::{monitor_event, loop_task_data};
use log::*;
use clap::{load_yaml, App};
use server::loop_proof_data;

mod server;
mod chain;
use crate::{server::start_rpc_server, chain::{PRIV_KEY, SCHEDULER_URL, CONTRACT, get_current_block_num}};


#[macro_use]
mod app_marco;

pub async fn process_proof_data() {
    loop{
        thread::sleep(time::Duration::from_secs(3));
        match loop_proof_data().await{
            Ok(()) => (),
            Err(_) => {
                info!("process proof data error occured")
            }
        }
    }
}

pub async fn process_task_data_loop() {
    loop{
        thread::sleep(time::Duration::from_secs(3));
        match loop_task_data().await{
            Ok(()) => (),
            Err(_) => {
                info!("process task data error occured")
            }
        }
    }
}

#[tokio::main]
async fn main() {
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .filter(Some("chain"), log::LevelFilter::Error)
    .init();

    let cli_param_yml = load_yaml!("app.yml");
    let cli_param = App::from_yaml(cli_param_yml).get_matches();

    let key: String = cli_param.value_of("key").unwrap_or("").into();
    let api: String = cli_param.value_of("api").unwrap_or("").into();
    let scheduler: String = cli_param.value_of("scheduler").unwrap_or("").into();
    let blk_num: String = cli_param.value_of("start_num").unwrap_or("").into();
    let contract_addr: String = cli_param.value_of("contract").unwrap_or("").into();

    {
        let mut priv_key = PRIV_KEY.lock().await;
        *priv_key=key.clone();
    
        let mut scheduler_url = SCHEDULER_URL.lock().await;
        *scheduler_url=scheduler.clone();

        let mut contract = CONTRACT.lock().await;
        *contract=contract_addr.clone();

    }
    
    let myserver = start_rpc_server(api);

    let srv_handle = tokio::spawn(async move {
        myserver.await.wait();
    });

    let event_loop_handle = tokio::spawn(async move {
        if blk_num.parse::<u64>().unwrap()==0 {
            let latest_blk = match get_current_block_num(){
                Some(r) => r,
                None => 0,
            };
            monitor_event(latest_blk).await

         }else {
            monitor_event(blk_num.parse::<u64>().unwrap()).await
        }
    });

    let process_proof_handle = tokio::spawn(async move {
        process_proof_data().await
    });
    let process_task_handle = tokio::spawn(async move {
        process_task_data_loop().await
    });

    tokio::select! {
       _ = async { srv_handle.await } => {
        info!("Server terminal")
        },
      _ = async { process_proof_handle.await } => {
        info!("process proof handle terminal")
       },
       _ = async { process_task_handle.await } => {
        info!("process task handle terminal")
       },
       _ = async { event_loop_handle.await } => {
        info!("process event loop handle terminal")
       },
    }
}
