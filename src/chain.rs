use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use ethereum_private_key_to_address::PrivateKey;
use log::*;
use tokio::sync::Mutex;
use web3::ethabi::FixedBytes;
use std::thread;
use std::time;
use core::str;
use rand::seq::SliceRandom;
use serde_derive::{Deserialize,Serialize};

use reqwest::{Client, Url};
use http_req::request::{Request, Method};
use http_req::uri::Uri;

use lazy_static::lazy_static;
use std::collections::VecDeque;

use web3::{
    Web3,
    ethabi,
    transports::Http,
    types,
    ethabi::{ethereum_types::U256,Function, ParamType, Param, StateMutability, Token},
    types::{Address,Bytes, TransactionParameters}, 
};

use web3::types::BlockNumber::Pending;
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static! {
    pub static ref PROOF_MSG_QUEUE: Arc<tokio::sync::Mutex<VecDeque<ProofMessage>>> = {
        Arc::new(tokio::sync::Mutex::new(VecDeque::new()))
    };
    pub static ref TASK_MSG_QUEUE: Arc<tokio::sync::Mutex<VecDeque<ProvenTaskMessage>>> = {
        Arc::new(tokio::sync::Mutex::new(VecDeque::new()))
    };
    pub static ref OPEN_TASK_MSG_QUEUE: Arc<tokio::sync::Mutex<VecDeque<ProvenTaskMessage>>> = {
        Arc::new(tokio::sync::Mutex::new(VecDeque::new()))
    };
    pub static ref PRIV_KEY: tokio::sync::Mutex<String> = {      //priv_key
        tokio::sync::Mutex::new(String::from(""))
    };
    pub static ref CONTRACT: tokio::sync::Mutex<String> = {      //contract
      tokio::sync::Mutex::new(String::from(""))
    };
    pub static ref SCHEDULER_URL: tokio::sync::Mutex<String> = {   //scheduler rpc url
        tokio::sync::Mutex::new(String::from(""))
    };
    pub static ref TASK_KEY_CACHE: Arc<Mutex<HashMap<String, String>>> = {
      Arc::new(Mutex::new(HashMap::default()))
    };
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProofMessage {
    pub task_id: String,
    pub proof:   String,
    pub degree:  String,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct EmitProvenTaskMessage {
    pub requester: String,
    pub prover: String,
    pub instance: String,
    pub task_key: String,
    pub reward_token: String,
    pub reward_amount: String,
    pub liability_window: String,
    pub liability_token: String,
    pub liability_amount: String,
   
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct ProvenTaskMessage {
    pub instance: String,
    pub task_key: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<String>,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    result: String,
    id: u64,
}

use crate::ok_or_continue;

pub const SEPOLIA_CHAIN_URLS: [&str; 1] = [
    "https://eth-sepolia.g.alchemy.com/v2/kMO8lL7g44IJOGR-Om-xxxxxxxxxxxxxx",
];

///TBD
pub const  GAS_UPPER : &str = "1000000";
const BLOCK_NUM_BODY: &[u8] = br#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":83}"#;


const MATIC_CONTRACT_ABI:&[u8] = r#"[
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "_owner",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "_bondToken",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "_bondAmount",
          "type": "uint256"
        },
        {
          "internalType": "address",
          "name": "_verifierAddress",
          "type": "address"
        },
        {
          "internalType": "uint32",
          "name": "_proofWindow",
          "type": "uint32"
        },
        {
          "internalType": "uint8",
          "name": "_instanceLength",
          "type": "uint8"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "constructor"
    },
    {
      "inputs": [],
      "name": "INVALID_ASSIGNMENT",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "INVALID_PROOF",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "INVALID_PROVER",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "INVALID_PROVER_SIG",
      "type": "error"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "owner",
          "type": "address"
        }
      ],
      "name": "OwnableInvalidOwner",
      "type": "error"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "account",
          "type": "address"
        }
      ],
      "name": "OwnableUnauthorizedAccount",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "ReentrancyGuardReentrantCall",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_ALREADY_OPEN",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_ALREADY_PROVEN",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_ALREADY_SUBMITTED",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_NONE_EXIST",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_NOT_OPEN",
      "type": "error"
    },
    {
      "inputs": [],
      "name": "TASK_NOT_THE_SAME",
      "type": "error"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "from",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        }
      ],
      "name": "BondDeposited",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "to",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "amount",
          "type": "uint256"
        }
      ],
      "name": "BondReleased",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "previousOwner",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "newOwner",
          "type": "address"
        }
      ],
      "name": "OwnershipTransferred",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "prover",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        }
      ],
      "name": "TaskProven",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "requester",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "prover",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "bytes",
          "name": "instance",
          "type": "bytes"
        },
        {
          "indexed": false,
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        },
        {
          "indexed": false,
          "internalType": "address",
          "name": "rewardToken",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "rewardAmount",
          "type": "uint256"
        },
        {
          "indexed": false,
          "internalType": "uint64",
          "name": "liabilityWindow",
          "type": "uint64"
        },
        {
          "indexed": false,
          "internalType": "address",
          "name": "liabilityToken",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "liabilityAmount",
          "type": "uint256"
        }
      ],
      "name": "TaskSubmitted",
      "type": "event"
    },
    {
      "anonymous": false,
      "inputs": [
        {
          "indexed": true,
          "internalType": "address",
          "name": "from",
          "type": "address"
        },
        {
          "indexed": true,
          "internalType": "address",
          "name": "to",
          "type": "address"
        },
        {
          "indexed": false,
          "internalType": "uint256",
          "name": "value",
          "type": "uint256"
        }
      ],
      "name": "Transfer",
      "type": "event"
    },
    {
      "inputs": [],
      "name": "owner",
      "outputs": [
        {
          "internalType": "address",
          "name": "",
          "type": "address"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        },
        {
          "internalType": "bytes",
          "name": "proof",
          "type": "bytes"
        }
      ],
      "name": "proveTask",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        }
      ],
      "name": "readProofStatus",
      "outputs": [
        {
          "components": [
            {
              "internalType": "bytes",
              "name": "instance",
              "type": "bytes"
            },
            {
              "internalType": "address",
              "name": "prover",
              "type": "address"
            },
            {
              "internalType": "uint64",
              "name": "submittedAt",
              "type": "uint64"
            },
            {
              "internalType": "bool",
              "name": "proven",
              "type": "bool"
            }
          ],
          "internalType": "struct TaskStatus",
          "name": "taskStatus",
          "type": "tuple"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [],
      "name": "renounceOwnership",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes",
          "name": "instance",
          "type": "bytes"
        },
        {
          "internalType": "address",
          "name": "prover",
          "type": "address"
        },
        {
          "internalType": "address",
          "name": "rewardToken",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "rewardAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint64",
          "name": "liabilityWindow",
          "type": "uint64"
        },
        {
          "internalType": "address",
          "name": "liabilityToken",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "liabilityAmount",
          "type": "uint256"
        },
        {
          "internalType": "uint64",
          "name": "expiry",
          "type": "uint64"
        },
        {
          "internalType": "bytes",
          "name": "signature",
          "type": "bytes"
        }
      ],
      "name": "submitTask",
      "outputs": [
        {
          "internalType": "bytes32",
          "name": "taskKey",
          "type": "bytes32"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "bytes32",
          "name": "",
          "type": "bytes32"
        }
      ],
      "name": "taskStatusMap",
      "outputs": [
        {
          "internalType": "bytes",
          "name": "instance",
          "type": "bytes"
        },
        {
          "internalType": "address",
          "name": "prover",
          "type": "address"
        },
        {
          "internalType": "uint64",
          "name": "submittedAt",
          "type": "uint64"
        },
        {
          "internalType": "bool",
          "name": "proven",
          "type": "bool"
        }
      ],
      "stateMutability": "view",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "newOwner",
          "type": "address"
        }
      ],
      "name": "transferOwnership",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    },
    {
      "inputs": [
        {
          "internalType": "address",
          "name": "_bondToken",
          "type": "address"
        },
        {
          "internalType": "uint256",
          "name": "_bondAmount",
          "type": "uint256"
        },
        {
          "internalType": "address",
          "name": "_verifierAddress",
          "type": "address"
        },
        {
          "internalType": "uint32",
          "name": "_proofWindow",
          "type": "uint32"
        },
        {
          "internalType": "uint8",
          "name": "_instanceLength",
          "type": "uint8"
        }
      ],
      "name": "updateConfig",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ]"#.as_bytes();


pub fn get_current_block_num() -> Option<u64>{
    let mut writer1 = Vec::new(); 
    let mut blocknums = vec![];
    loop{

        let url_str = match SEPOLIA_CHAIN_URLS.choose(&mut rand::thread_rng()){
            Some(url) => url.to_string(),
            None => continue
          };
      
        writer1.clear();
        let uri: Uri = Uri::try_from(url_str.as_str()).unwrap();
        let res= match Request::new(&uri)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Content-Length", &BLOCK_NUM_BODY.len())
            .body(BLOCK_NUM_BODY)
            .send(&mut writer1){
                Ok(r) => { r },
                Err(e) => {
                    error!("get block num :{}", e);
                    continue;
                },
            };
            if !res.status_code().is_success() {
                continue;
            }
        let content = str::from_utf8(&writer1).unwrap();
        let v: RpcResponse = match serde_json::from_str(content){
            Ok(r) => { r },
            Err(e) => {
                error!("get block num :{}", e);
                continue;
            },
        };
        blocknums.push(u64::from_str_radix(v.result.trim_start_matches("0x"), 16).unwrap());
        if blocknums.len() >= 4 {
            return Some(*blocknums.iter().min().unwrap());
        }  
    }
}


/// get the account nonce value
pub async fn get_nonce(address:String) -> U256{
    loop {
        for url in SEPOLIA_CHAIN_URLS.iter() {
            let transport = match web3::transports::Http::new(&url){
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },
            };
            let web3 = web3::Web3::new(transport);
            let addr_addr:Address=Address::from_str(address.as_str()).unwrap(); 
            let nonce= match web3.eth().transaction_count(addr_addr,Some(Pending)).await{
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },  
            };
            debug!("nonce value is {:?}",nonce.clone());
            return nonce
    }
 }
}

/// 1.1 multiple of the network gas
pub async fn gas_price() -> U256{
    loop {
        for url in SEPOLIA_CHAIN_URLS.iter() {
            let transport = match web3::transports::Http::new(&url){
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },
            };
            let web3 = web3::Web3::new(transport);
            let gas_price= match web3.eth().gas_price().await{
                Ok(r) => r,
                Err(_) => continue,
            };

            let upper_gas = (gas_price.as_u64())*(100)/(90)*(10);
            info!("gas price value is {:?}",upper_gas.clone());
            return U256::from_dec_str(&upper_gas.to_string()).unwrap()      
    }
 }
}

/// submit proof data to sepolia chain
pub async fn submit_proof(
    task_key:FixedBytes,
    proof:Bytes,
) -> Result<String, String> { 

    let url_str = SEPOLIA_CHAIN_URLS.choose(&mut rand::thread_rng()).unwrap();
    let transport = web3::transports::Http::new(url_str).unwrap();
    let web3 = web3::Web3::new(transport);

    let ctr = CONTRACT.lock().await;
    let ctr_addr = (*ctr).clone();
    let contract_address = Address::from_str(ctr_addr.as_str()).unwrap();
    
    let func = Function {
        name: "proveTask".to_owned(),
        inputs: vec![
            Param { name: "taskKey".to_owned(), kind: ParamType::FixedBytes(32), internal_type: None },
            Param { name: "proof ".to_owned(), kind: ParamType::Bytes, internal_type: None },
        ],
        outputs: vec![],
        constant: Some(false),
        state_mutability: StateMutability::Payable,
    };

      //enocde send tx input parameters
    let mut data_vec_input:Vec<Token>=Vec::new();
    data_vec_input.push(Token::FixedBytes(task_key));
    data_vec_input.push(Token::Bytes(proof.0));

    let tx_data = match func.encode_input(&data_vec_input){
        Ok(r) => r,
        Err(e) => {
          info!("encode input error:{:?}",e);
          return Err("err".to_string())
        },
    };
    let priv_key = PRIV_KEY.lock().await;
    let key = (*priv_key).clone();
    let prvk = web3::signing::SecretKey::from_str(key.as_str()).unwrap();
    let private_key = PrivateKey::from_str(key.as_str()).unwrap();

    let tx_object = TransactionParameters {
        to: Some(contract_address),
        gas_price:Some(gas_price().await), 
        gas:U256::from_dec_str(GAS_UPPER).unwrap(),
        nonce:Some(get_nonce(private_key.address()).await),
        data:Bytes(tx_data),
        ..Default::default()
    };
        //send tx to network
    loop {
        let signed = match web3.accounts().sign_transaction(tx_object.clone(), &prvk).await {
            Ok(r) => {
                r
            },
            Err(_) => {
                continue
            }
        };

        let result = match web3.eth().send_raw_transaction(signed.raw_transaction).await{
            Ok(r )=> r,
            Err(e) => {
                return Err(e.to_string())
            },
        };
            
        debug!("invoke a tx hash is : {:?}",result);
        return Ok(hex::encode(result.as_bytes()))
    }
}

macro_rules! ok_or_return{
    ($exec: expr, $content: literal)=>{
        match $exec {
            Ok(value) => value,
            Err(e) => {
                error!("{} failed: {}", $content, e);
                return Err(web3::error::Error::Internal);
            }
        }
    };
  }



pub async fn monitor_event(start_block_num:u64) { 
    info!("loop scan block event progrom begin start block:{}",start_block_num);
    let mut handle_block_num =start_block_num;
    'outer: loop{
        let url_str = match SEPOLIA_CHAIN_URLS.choose(&mut rand::thread_rng()){
            Some(url) => url,
            None => continue
          };

        let mut builder = Client::builder();
        builder = builder.user_agent(headers::HeaderValue::from_static("web3.rs"));
        builder = builder.timeout(std::time::Duration::from_secs(30));
        let client = ok_or_continue!(builder.build(), "build web3 client");
        let url: Url = ok_or_continue!(url_str.parse(), format!("url.parse() {}", url_str));
        let web3 = Web3::new(Http::with_client(client, url.clone()));

        let world_num = ok_or_continue!(web3.eth().block_number().await, "world_num web3.eth().block_number()").as_u64();
        let mut start_num = 0;
        let mut end_num = start_num;
        if handle_block_num < world_num {
          start_num = handle_block_num + 1;
          end_num = if world_num - handle_block_num > 10 { handle_block_num + 10 } else { world_num };
        }else {
            thread::sleep(time::Duration::from_secs(2));
            continue;
        }

        let matic_contract = ok_or_continue!(ethabi::Contract::load(MATIC_CONTRACT_ABI), "ethabi::Contract::load MATIC_CONTRACT_ABI");

        let event_insert = ok_or_continue!(matic_contract.event("TaskSubmitted"), "matic_contract.event Insert");
        let topic_insert_filter = ok_or_continue!(event_insert.filter(ethabi::RawTopicFilter {
            topic0: ethabi::Topic::Any,
            topic1: ethabi::Topic::Any,
            topic2: ethabi::Topic::Any,
        }), "event_insert.filter");

        let ctr = CONTRACT.lock().await;
        let ctr_addr = (*ctr).clone();

        let contract_addr_hex = ok_or_continue!(hex::decode(ctr_addr.as_str()), "rsic0");
        if let ethabi::Topic::This(topic1) = topic_insert_filter.topic0 {
        let filter = types::FilterBuilder::default()
            .address(vec![ethabi::ethereum_types::H160::from_slice(&contract_addr_hex)])
            .topics(
                Some(vec![topic1]),
                None,
                None,
                None,
            ).from_block(types::BlockNumber::Number(start_num.into()))
            .to_block(types::BlockNumber::Number(end_num.into()))
            .build();
        let logs = ok_or_continue!(web3.eth().logs(filter).await, "web3.eth().logs(filter)");
        info!("current process from {} to {}", start_num, end_num);
        for log in logs.iter(){    
            let mut temp = EmitProvenTaskMessage::default(); 
    
            if log.topics.get(0).eq(&Some(&topic1)){
              ok_or_continue!(add_proof_info(event_insert, &mut temp, log).await, "add_info insert", continue 'outer);
            }
          }
        }
        //update the handled block num
        handle_block_num=end_num;
    }
}

pub async fn add_proof_info(event: &ethabi::Event, temp: &mut EmitProvenTaskMessage, log: &types::Log) -> web3::Result{ 
    let info = ok_or_return!(event.parse_log(ethabi::RawLog{
      topics: log.topics.clone(),
      data: log.data.0.clone()
    }), "event.parse_log");
  
    for item in info.params.iter(){
      match &item.value{
        ethabi::Token::Address(content) => {
          if item.name.eq("requester") {
            temp.requester = hex::encode(content.as_bytes());
          }else if item.name.eq("prover"){
            temp.prover = hex::encode(content.as_bytes());
          }else if item.name.eq("rewardToken"){
            temp.reward_token = hex::encode(content.as_bytes());
          }else if item.name.eq("liabilityToken"){
            temp.liability_token = hex::encode(content.as_bytes());
          }
        },
        ethabi::Token::Uint(content) => {
            if item.name.eq("rewardAmount"){
              temp.reward_amount = content.to_string();
            }else if item.name.eq("liabilityWindow"){
              temp.liability_window = content.to_string();
            }
            else if item.name.eq("liabilityAmount"){
                temp.liability_amount = content.to_string();
              }
          },
          ethabi::Token::Bytes(content) => {
            temp.instance = hex::encode(content);
          },
          ethabi::Token::FixedBytes(content) => {
            temp.task_key=hex::encode(content);
        },
        _ => {
          continue;
        }
      }
    }
    info!("receive the proof info need to be proven :{:?}",temp);
    receive_task(temp.instance.clone(), temp.task_key.clone()).await;
    Ok(())
  }


///no need to verify onchain
pub async fn process_proof_data(msg: &ProofMessage){  //TBD

  //   let task_key_temp = TASK_KEY_CACHE.clone();
  //   let task_key_map = task_key_temp.lock().await;

  //   let task_key = match task_key_map.get(&msg.task_id) {        
  //     Some(r) => {
  //         r.clone()
  //     },
  //     None => {
  //       info!("can not find the task_key according to the task_id,ignore it");
  //       return ;
  //     },
  //  };

    match submit_proof(hex::decode(msg.task_id.clone()).unwrap(), Bytes::from(msg.proof.clone())).await{
        Ok(r) => {
            info!("****** sbumit task_key:{} proof tx success,tx hash is: {}",msg.task_id,r)
        },
        Err(_) => {
            error!("sbumit proof tx failed")
        },
    };
}

pub async fn receive_task(instance:String,task_key:String){
    info!("receive onchain task info data: {}-{},add to queue",instance,task_key);
    let msg:ProvenTaskMessage=ProvenTaskMessage { instance,task_key };
    let mut queue = TASK_MSG_QUEUE.lock().await;
    queue.push_back(msg);
}

pub async fn loop_task_data() -> web3::Result<()> {
    let mut queue = TASK_MSG_QUEUE.lock().await;
    while queue.len() > 0 {
        info!(" start to process the task data queue len :{}",queue.len());
        let item = queue.pop_front().unwrap();
        process_task_data(&item).await;
    }
    Ok(())
}

///send the task to scheduler
///TBD(how to generate task_id)?
pub async fn  process_task_data(msg: &ProvenTaskMessage)-> bool{  
    // let task_id = SystemTime::now()
    // .duration_since(UNIX_EPOCH)
    // .unwrap()
    // .as_millis(); //generate random task_id 

    //cache the task_key according to its task_id
    // let task_key_temp = TASK_KEY_CACHE.clone();
    // let mut task_key_map = task_key_temp.lock().await;
    // task_key_map.insert((task_id%10000).to_string(), msg.task_key.clone());

    let request = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "DelieveTask".to_string(),
        // params: vec!["demo".to_string(),(task_id%10000).to_string(),msg.instance.clone(),"1".to_string()], 
        params: vec!["demo".to_string(),msg.task_key.clone(),msg.instance.clone(),"1".to_string()], 
        id: "1".to_string(),
    };

    let mut writer_buffer = Vec::new(); 
    let mut retry = 0 ;

    let scheduler_url = SCHEDULER_URL.lock().await;
    let scheduler_endpoint = (*scheduler_url).clone();
    info!("try to send task key:{:?},proof task:{:?} to scheduler service:{:?}",msg.task_key.clone(),msg.instance,scheduler_endpoint.clone());
    loop{
        if retry==1 {
            error!("send task to scheduler error of msg :{:?}", msg.clone());
            return false
        }
        writer_buffer.clear();

        let parameter_string=serde_json::to_string(&request).unwrap();

        let uri: Uri = Uri::try_from(scheduler_endpoint.as_str()).unwrap();
        let _res= match Request::new(&uri)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Content-Length", &parameter_string.as_bytes().len())
            .body(parameter_string.as_bytes())
            .send(&mut writer_buffer){
                Ok(r) => { r },
                Err(_) => {
                    retry=retry+1;
                    continue;
                },
            };
        let content = str::from_utf8(&writer_buffer).unwrap();
        info!("send result is {:?}",content);
        return true
    }
}






  
  
