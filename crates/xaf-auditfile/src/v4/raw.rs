//! Structures that follow `XmlAuditfileFinancieel4.0.xsd` one to one. Every value is still the
//! text from the file; interpretation happens in the mapping to the public model, so that errors
//! can name the element and transaction they belong to.

use quick_xml::DeError;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Error;

pub(super) fn deserialize<T: DeserializeOwned>(xml: &[u8]) -> Result<T, Error> {
    quick_xml::de::from_reader(xml).map_err(|error| match error {
        DeError::InvalidXml(error) => Error::from(error),
        error => Error::Deserialize(error),
    })
}

#[derive(Debug, Deserialize)]
pub(super) struct Auditfile {
    pub header: Header,
    pub company: Company,
}

#[derive(Debug, Deserialize)]
pub(super) struct Header {
    #[serde(rename = "fiscalYear")]
    pub fiscal_year: String,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    #[serde(rename = "curCode")]
    pub cur_code: String,
    #[serde(rename = "dateCreated")]
    pub date_created: String,
    #[serde(rename = "softwareDesc")]
    pub software_desc: String,
    #[serde(rename = "softwareVersion")]
    pub software_version: String,
    #[serde(rename = "RGSVersion")]
    pub rgs_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Company {
    #[serde(rename = "Commercenr")]
    pub commerce_nr: Option<String>,
    #[serde(rename = "companyName")]
    pub company_name: String,
    #[serde(rename = "taxRegistrationCountry")]
    pub tax_registration_country: String,
    #[serde(rename = "taxRegIdent")]
    pub tax_reg_ident: String,
    #[serde(rename = "streetAddress", default)]
    pub street_address: Vec<StreetAddress>,
    #[serde(rename = "customersSuppliers")]
    pub customers_suppliers: Option<CustomersSuppliers>,
    #[serde(rename = "generalLedger")]
    pub general_ledger: Option<GeneralLedger>,
    #[serde(rename = "vatCodes")]
    pub vat_codes: Option<VatCodes>,
    pub periods: Option<Periods>,
    #[serde(rename = "openingBalance")]
    pub opening_balance: Option<OpeningBalance>,
    pub transactions: Option<Transactions>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StreetAddress {
    pub streetname: Option<String>,
    pub number: Option<String>,
    #[serde(rename = "numberExtension")]
    pub number_extension: Option<String>,
    pub city: Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CustomersSuppliers {
    #[serde(rename = "customerSupplier", default)]
    pub customer_supplier: Vec<CustomerSupplier>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CustomerSupplier {
    #[serde(rename = "custSupID")]
    pub cust_sup_id: String,
    #[serde(rename = "custSupName")]
    pub cust_sup_name: Option<String>,
    #[serde(rename = "eMail")]
    pub e_mail: Option<String>,
    #[serde(rename = "commerceNr")]
    pub commerce_nr: Option<String>,
    #[serde(rename = "taxRegistrationCountry")]
    pub tax_registration_country: Option<String>,
    #[serde(rename = "taxRegIdent")]
    pub tax_reg_ident: Option<String>,
    #[serde(rename = "custSupTp")]
    pub cust_sup_tp: Option<String>,
    #[serde(rename = "opBalDesc")]
    pub op_bal_desc: Option<String>,
    #[serde(rename = "opBalTp")]
    pub op_bal_tp: Option<String>,
    #[serde(rename = "clBalDesc")]
    pub cl_bal_desc: Option<String>,
    #[serde(rename = "clBalTp")]
    pub cl_bal_tp: Option<String>,
    #[serde(rename = "streetAddress", default)]
    pub street_address: Vec<StreetAddress>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GeneralLedger {
    #[serde(rename = "ledgerAccount")]
    pub ledger_account: Vec<LedgerAccount>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LedgerAccount {
    #[serde(rename = "accID")]
    pub acc_id: String,
    #[serde(rename = "accDesc")]
    pub acc_desc: String,
    #[serde(rename = "accTp")]
    pub acc_tp: String,
    #[serde(rename = "RGScode")]
    pub rgs_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct VatCodes {
    #[serde(rename = "vatCode", default)]
    pub vat_code: Vec<VatCode>,
}

#[derive(Debug, Deserialize)]
pub(super) struct VatCode {
    #[serde(rename = "vatID")]
    pub vat_id: String,
    #[serde(rename = "vatDesc")]
    pub vat_desc: String,
    #[serde(rename = "vatToPayAccID")]
    pub vat_to_pay_acc_id: Option<String>,
    #[serde(rename = "vatToClaimAccID")]
    pub vat_to_claim_acc_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Periods {
    #[serde(default)]
    pub period: Vec<Period>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Period {
    #[serde(rename = "periodNumber")]
    pub period_number: String,
    #[serde(rename = "startDatePeriod")]
    pub start_date_period: String,
    #[serde(rename = "endDatePeriod")]
    pub end_date_period: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpeningBalance {
    #[serde(rename = "linesCount")]
    pub lines_count: String,
    #[serde(rename = "totalDebit")]
    pub total_debit: String,
    #[serde(rename = "totalCredit")]
    pub total_credit: String,
    #[serde(rename = "obLine", default)]
    pub ob_line: Vec<ObLine>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ObLine {
    pub nr: String,
    #[serde(rename = "accID")]
    pub acc_id: String,
    pub amnt: String,
    #[serde(rename = "amntTp")]
    pub amnt_tp: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Transactions {
    #[serde(rename = "linesCount")]
    pub lines_count: String,
    #[serde(rename = "totalDebit")]
    pub total_debit: String,
    #[serde(rename = "totalCredit")]
    pub total_credit: String,
    #[serde(default)]
    pub journal: Vec<Journal>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Journal {
    #[serde(rename = "jrnID")]
    pub jrn_id: String,
    pub desc: String,
    #[serde(rename = "jrnTp")]
    pub jrn_tp: Option<String>,
    #[serde(rename = "offsetAccID")]
    pub offset_acc_id: Option<String>,
    #[serde(default)]
    pub transaction: Vec<Transaction>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Transaction {
    pub nr: String,
    pub desc: Option<String>,
    #[serde(rename = "periodNumber")]
    pub period_number: String,
    #[serde(rename = "trDt")]
    pub tr_dt: String,
    #[serde(rename = "Source")]
    pub source: Option<String>,
    #[serde(rename = "User")]
    pub user: Option<String>,
    #[serde(rename = "trLine", default)]
    pub tr_line: Vec<TrLine>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TrLine {
    pub nr: String,
    #[serde(rename = "accID")]
    pub acc_id: String,
    #[serde(rename = "docRef")]
    pub doc_ref: String,
    #[serde(rename = "effDate")]
    pub eff_date: String,
    #[serde(rename = "settDate")]
    pub sett_date: Option<String>,
    pub desc: Option<String>,
    pub amnt: String,
    #[serde(rename = "amntTp")]
    pub amnt_tp: String,
    #[serde(rename = "custSupID")]
    pub cust_sup_id: Option<String>,
    #[serde(rename = "invRef")]
    pub inv_ref: Option<String>,
    #[serde(rename = "receivingDocRef")]
    pub receiving_doc_ref: Option<String>,
    #[serde(rename = "shipDocRef")]
    pub ship_doc_ref: Option<String>,
    pub cost: Option<String>,
    pub product: Option<String>,
    pub project: Option<String>,
    #[serde(rename = "workCostArrRef")]
    pub work_cost_arr_ref: Option<String>,
    #[serde(rename = "bankAccNr")]
    pub bank_acc_nr: Option<String>,
    #[serde(rename = "offsetBankAccNr")]
    pub offset_bank_acc_nr: Option<String>,
    #[serde(default)]
    pub vat: Vec<Vat>,
    pub currency: Option<Currency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Vat {
    #[serde(rename = "vatID")]
    pub vat_id: String,
    #[serde(rename = "vatPerc")]
    pub vat_perc: String,
    #[serde(rename = "vatAmnt")]
    pub vat_amnt: String,
    #[serde(rename = "vatAmntTp")]
    pub vat_amnt_tp: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Currency {
    #[serde(rename = "curCode")]
    pub cur_code: String,
    #[serde(rename = "curAmnt")]
    pub cur_amnt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEBSHOP: &[u8] = include_bytes!("../../tests/fixtures/webshop.xaf");

    fn webshop() -> Auditfile {
        deserialize(WEBSHOP).unwrap()
    }

    #[test]
    fn reads_header_and_company() {
        let file = webshop();
        assert_eq!(file.header.fiscal_year, "2024");
        assert_eq!(file.header.software_desc, "Example Books");
        assert_eq!(file.header.rgs_version, None);
        assert_eq!(file.company.company_name, "Voorbeeld Webshop B.V.");
        assert_eq!(file.company.commerce_nr.as_deref(), Some("12345678"));
        assert_eq!(file.company.street_address.len(), 1);
    }

    #[test]
    fn reads_master_data() {
        let company = webshop().company;
        let relations = company.customers_suppliers.unwrap().customer_supplier;
        assert_eq!(relations.len(), 2);
        assert_eq!(relations[1].op_bal_desc.as_deref(), Some("50.00"));
        assert!(relations[1].street_address.is_empty());
        assert_eq!(company.general_ledger.unwrap().ledger_account.len(), 5);
        assert_eq!(company.vat_codes.unwrap().vat_code.len(), 2);
        assert_eq!(company.periods.unwrap().period.len(), 2);
        assert_eq!(company.opening_balance.unwrap().ob_line.len(), 2);
    }

    #[test]
    fn reads_journals_in_document_order() {
        let transactions = webshop().company.transactions.unwrap();
        assert_eq!(transactions.lines_count, "8");
        let ids: Vec<_> = transactions
            .journal
            .iter()
            .map(|j| j.jrn_id.as_str())
            .collect();
        assert_eq!(ids, ["VERK", "BANK"]);
        assert_eq!(transactions.journal[0].transaction.len(), 2);
        assert_eq!(transactions.journal[0].transaction[1].tr_line.len(), 3);
    }

    #[test]
    fn reads_multiple_vat_entries_on_one_line() {
        let transactions = webshop().company.transactions.unwrap();
        let line = &transactions.journal[0].transaction[1].tr_line[1];
        let vat: Vec<_> = line
            .vat
            .iter()
            .map(|v| (v.vat_id.as_str(), v.vat_perc.as_str(), v.vat_amnt.as_str()))
            .collect();
        assert_eq!(vat, [("HOOG", "21", "21.00"), ("LAAG", "9", "4.50")]);
    }

    #[test]
    fn reads_foreign_currency_and_missing_optionals() {
        let transactions = webshop().company.transactions.unwrap();
        let line = &transactions.journal[1].transaction[0].tr_line[0];
        let currency = line.currency.as_ref().unwrap();
        assert_eq!(
            (currency.cur_code.as_str(), currency.cur_amnt.as_str()),
            ("USD", "108.00")
        );
        assert_eq!(line.cost, None);
        assert!(line.vat.is_empty());
    }

    #[test]
    fn empty_element_is_visible_to_the_mapping() {
        let transactions = webshop().company.transactions.unwrap();
        let line = &transactions.journal[1].transaction[0].tr_line[0];
        assert_eq!(line.bank_acc_nr.as_deref(), Some(""));
    }

    #[test]
    fn missing_required_element_is_a_deserialize_error() {
        let xml = br#"<auditfile xmlns="x"><company/></auditfile>"#;
        assert!(matches!(
            deserialize::<Auditfile>(xml),
            Err(Error::Deserialize(_))
        ));
    }
}
