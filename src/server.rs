use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_http_server::ServerBuilder;
use log::info;
use serde_derive::{Deserialize,Serialize};
use web3::ethabi::{Token, encode};
use web3::signing::keccak256;

use ethers_signers::{Wallet,Signer};

use crate::chain::{get_current_block_num, ProofMessage, PROOF_MSG_QUEUE, process_proof_data, PRIV_KEY};
use std::str::FromStr;
use ethereum_private_key_to_address::PrivateKey;


#[derive(Debug, Serialize, Deserialize,Default)]
struct TaskResponse {
    pub prover: String,
    pub instance: String,
    pub reward_token: String,
    pub reward: u64,
    pub liability_window: u64,
    pub liability_token: String,
    pub liability: u64,
    pub expiry: u64,
    pub signature: String,
}

pub async fn start_rpc_server(addr:String) -> jsonrpc_http_server::Server {
    let mut io = IoHandler::default();

    io.add_method("ReceiveTask", |params: Params| async {
        info!("****** receive ReceiveTask msg ******");
        let req_input: Vec<Value> = match params.parse(){
            Ok(r) => r,
            Err(_) => {
                return Ok(Value::String("parameter invalid".to_string()))
            },
        };
        if req_input.len() != 6 {
            return Ok(Value::String("parameter invalid".to_string()))
        }

        //task param
        let task_instance = if  let Value::String(func_input)=req_input[0].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

        // proof window
        let liability_window = if  let Value::String(func_input)=req_input[1].clone(){
            func_input.parse::<u64>().unwrap()
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

        //liability_token
        let liability_token = if  let Value::String(func_input)=req_input[2].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

          //liability
          let liability = if  let Value::String(func_input)=req_input[3].clone(){
            func_input.parse::<u64>().unwrap()
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

         //reward token
         let reward_token = if  let Value::String(func_input)=req_input[4].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

         //reward
         let reward = if  let Value::String(func_input)=req_input[5].clone(){
            func_input.parse::<u64>().unwrap()
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

        let priv_key = PRIV_KEY.lock().await;
        let key = (*priv_key).clone();
        let private_key = PrivateKey::from_str(key.as_str()).unwrap();

        let mut res:TaskResponse=TaskResponse::default();
        res.prover=private_key.address();
        res.instance=task_instance.clone();
        res.reward_token=reward_token.clone();
        res.reward=reward;
        res.liability_window=liability_window;
        res.liability_token=liability_token.clone();
        res.liability=liability;
        let block_num = get_current_block_num().unwrap();
        res.expiry=block_num+2000;   //expire time + 2000 block time

        //encode ABI function array
        let mut data_vec:Vec<Token>=Vec::new();
        data_vec.push(Token::String(task_instance.clone()));
        data_vec.push(Token::String(reward_token.clone()));
        data_vec.push(Token::Uint(reward.clone().into()));
        data_vec.push(Token::String(liability_token.clone()));
        data_vec.push(Token::Uint(liability.clone().into()));
        data_vec.push(Token::Uint(res.expiry.into()));
        data_vec.push(Token::Uint(liability_window.into()));

        let encode_sig_msg=encode(&data_vec);
        let keccak_hash = keccak256(&encode_sig_msg);
        let wallet:Wallet=key.parse().unwrap();
        let signature = wallet.sign_message(&keccak_hash).to_string();
        res.signature=signature.to_string();
        Ok(Value::String(serde_json::to_string(&res).unwrap()))
        
    });

    io.add_method("demo/SendProofBack", |params: Params| async {
        info!("****** receive SendProofBack msg ******");
        let req_input: Vec<Value> = match params.parse(){
            Ok(r) => r,
            Err(_) => {
                return Ok(Value::String("parameter invalid".to_string()))
            },
        };
        if req_input.len() != 3 {
            return Ok(Value::String("parameter invalid".to_string()))
        }
        //block id
        let task_id = if  let Value::String(func_input)=req_input[0].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

        // zkproof
        let zkproof = if  let Value::String(func_input)=req_input[1].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        };

        //degree
        let degree = if  let Value::String(func_input)=req_input[2].clone(){
            func_input
        }else{
            return Ok(Value::String("parameter invalid".to_string()))
        }; 
        receive_proof(task_id, zkproof, degree).await;  
        Ok(Value::String("success".to_string()))
        
    }); 
    info!("start the server on :{}",addr.clone());
    let server = ServerBuilder::new(io)
        .threads(4)
        .start_http(&addr.parse().unwrap())
        .unwrap();
    
    server
}


pub async fn receive_proof(task_id:String,proof:String,degree:String){
    info!("receive scheduler proof info of {:?},data is {:?},add to queue",task_id,proof);
    let msg:ProofMessage=ProofMessage { task_id, proof, degree };
    let mut queue = PROOF_MSG_QUEUE.lock().await;
    queue.push_back(msg);
}

pub async fn loop_proof_data() -> web3::Result<()> {
    let mut queue = PROOF_MSG_QUEUE.lock().await;
    while queue.len() > 0 {
        info!("start to process the proof data of len : {}",queue.len());
        let item = queue.pop_front().unwrap();
        process_proof_data(&item).await;
    }
    Ok(())
}





