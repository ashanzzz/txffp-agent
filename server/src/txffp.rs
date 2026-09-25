use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Block};
use base64::Engine;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const AES_LOGIN_KEY: &[u8; 16] = b"#xy@etcchina.com";

pub fn encrypt_password(password: &str) -> String {
    let cipher = Aes128::new(AES_LOGIN_KEY.into());

    let bytes = password.as_bytes();
    let pad_len = 16 - (bytes.len() % 16);
    let mut padded = Vec::with_capacity(bytes.len() + pad_len);
    padded.extend_from_slice(bytes);
    padded.resize(bytes.len() + pad_len, pad_len as u8);

    for chunk in padded.chunks_exact_mut(16) {
        let block: &mut Block = chunk.try_into().expect("16-byte chunk");
        cipher.encrypt_block(block);
    }

    base64::engine::general_purpose::STANDARD.encode(&padded)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcCard {
    pub card_id: String,
    pub card_no_masked: String,
    pub card_type: String,
    pub plate_number: String,
    pub balance: Option<Decimal>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub record_id: String,
    pub card_id: String,
    pub plate_number: String,
    pub en_time: String,
    pub ex_time: String,
    pub en_station: String,
    pub ex_station: String,
    pub amount: Decimal,
    pub invoice_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceTitle {
    pub title_id: String,
    pub title_name: String,
    pub tax_no: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePreviewRequest {
    pub start_date: String,
    pub end_date: String,
    pub card_id: Option<String>,
    pub invoice_title_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePreviewResponse {
    pub start_date: String,
    pub end_date: String,
    pub total_records: usize,
    pub invoiceable_records: usize,
    pub total_amount: Decimal,
    pub card_id: Option<String>,
    pub title_name: Option<String>,
    pub records: Vec<TransactionRecord>,
    pub can_submit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub start_date: String,
    pub end_date: String,
    pub card_id: Option<String>,
    pub invoice_title_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceResponse {
    pub invoice_id: String,
    pub status: String,
    pub message: String,
    pub total_amount: Decimal,
    pub record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub invoice_id: String,
    pub invoice_code: Option<String>,
    pub invoice_number: Option<String>,
    pub amount: Decimal,
    pub issue_date: String,
    pub status: String,
    pub pdf_download_url: Option<String>,
    pub summary_download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: String,
    pub invoice_id: String,
    pub file_type: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_decimal_precision() {
        let d1 = Decimal::from_str("25.50").unwrap();
        let d2 = Decimal::from_str("32.00").unwrap();
        let total = d1 + d2;
        assert_eq!(total.to_string(), "57.50");
    }

    #[test]
    fn test_password_aes_encryption() {
        let encrypted = encrypt_password("MyPassword123");
        assert_eq!(encrypted, "6aGTJZUW9WSNGEPbX9Kj/g==");
    }
}
