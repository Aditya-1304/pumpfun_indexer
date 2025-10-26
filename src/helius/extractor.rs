use crate::database::model::GeneralTransaction;
use solana_transaction_status::{EncodedTransactionWithStatusMeta, UiMessage};
use chrono::{DateTime, Utc};
use anyhow::Result;

pub fn extract_transaction_metadata(
    signature: &str,
    slot: u64,
    transaction: &EncodedTransactionWithStatusMeta,
    block_time: Option<i64>,
) -> Result<GeneralTransaction> {

    let block_time_dt = if let Some(time) = block_time {
        DateTime::from_timestamp(time, 0)
            .unwrap_or_else(|| Utc::now())
    } else {
        Utc::now()
    };

    let fee = if let Some(meta) = &transaction.meta {
        meta.fee
    } else {
        0
    };

    let (success, error_message) = if let Some(meta) = &transaction.meta {
        let is_success = meta.err.is_none();
        let err_msg = meta.err.as_ref().map(|e| format!("{:?}", e));
        (is_success, err_msg)
    } else {
        (false, None)
    };

    let (signer, accounts_involved, instruction_count) = match &transaction.transaction {
        solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
            
            match &ui_tx.message {
                UiMessage::Parsed(parsed_msg) => {
                    
                    let account_keys: Vec<String> = parsed_msg.account_keys.iter()
                        .map(|key| key.pubkey.clone())
                        .collect();
                    
                    let signer_str = if !account_keys.is_empty() {
                        account_keys[0].clone()
                    } else {
                        "unknown".to_string()
                    };
                    
                    let instruction_count = parsed_msg.instructions.len() as i32;
                    
                    (signer_str, account_keys, instruction_count)
                }
                UiMessage::Raw(raw_msg) => {
                    
                    let account_keys: Vec<String> = raw_msg.account_keys.iter()
                        .map(|key| key.clone())
                        .collect();
                    
                    let signer_str = if !account_keys.is_empty() {
                        account_keys[0].clone()
                    } else {
                        "unknown".to_string()
                    };
                    
                    let instruction_count = raw_msg.instructions.len() as i32;
                    
                    (signer_str, account_keys, instruction_count)
                }
            }
        }
        solana_transaction_status::EncodedTransaction::LegacyBinary(_) |
        solana_transaction_status::EncodedTransaction::Binary(_, _) |
        solana_transaction_status::EncodedTransaction::Accounts(_) => {
           
            if let Some(decoded_tx) = transaction.transaction.decode() {
                let account_keys = decoded_tx.message.static_account_keys();
                let signer_str = if !account_keys.is_empty() {
                    account_keys[0].to_string()
                } else {
                    "unknown".to_string()
                };
                
                let all_accounts: Vec<String> = account_keys.iter()
                    .map(|key| key.to_string())
                    .collect();
                
                let instruction_count = decoded_tx.message.instructions().len() as i32;
                
                (signer_str, all_accounts, instruction_count)
            } else {
                ("unknown".to_string(), vec![], 0)
            }
        }
    };

    
    let (pre_balances, post_balances) = if let Some(meta) = &transaction.meta {
        let pre = meta.pre_balances.iter().map(|&b| b as i64).collect();
        let post = meta.post_balances.iter().map(|&b| b as i64).collect();
        (pre, post)
    } else {
        (vec![], vec![])
    };

    
    let compute_units = if let Some(meta) = &transaction.meta {
        match &meta.compute_units_consumed {
            solana_transaction_status::option_serializer::OptionSerializer::Some(units) => {
                Some(*units as i64)
            }
            _ => None
        }
    } else {
        None
    };

    
    let (log_count, has_program_data) = if let Some(meta) = &transaction.meta {
        match &meta.log_messages {
            solana_transaction_status::option_serializer::OptionSerializer::Some(logs) => {
                let count = logs.len() as i32;
                let has_data = logs.iter().any(|log| log.contains("Program data:"));
                (count, has_data)
            }
            _ => (0, false)
        }
    } else {
        (0, false)
    };

    Ok(GeneralTransaction {
        signature: signature.to_string(),
        slot,
        block_time: block_time_dt,
        fee,
        success,
        signer,
        instruction_count,
        log_messages_count: log_count,
        has_program_data,
        accounts_involved,
        pre_balances,
        post_balances,
        compute_units_consumed: compute_units,
        error_message,
    })
}