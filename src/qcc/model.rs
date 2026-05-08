use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompanyInfo {
    pub name: String,
    pub tags: String,
    pub status: String,
    pub legal_person: String,
    pub registered_capital: String,
    pub detail_url: String,
    pub established_date: String,
    pub shareholder: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompanyDetail {
    pub name: String,
    pub status: String,
    pub description: String,
    pub industry: String,
    pub scale: String,
    pub employee_count: String,
    pub insurance_count: String,
    pub business_scope: String,
    pub established_date: String,
    pub registered_capital: String,
    pub legal_person: String,
    pub financing_stage: String,
    pub phone: String,
    pub website: String,
    pub address: String,
}
