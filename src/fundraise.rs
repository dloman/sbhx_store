use actix_web::{web, HttpResponse};
use braintree::{Braintree};
use serde::{Serialize, Deserialize};
use log::{debug, error, info};
use std::collections::BTreeMap;
use std::fs::File;
use std::sync::{Mutex};

use crate::util;

#[derive(Deserialize,Debug, Serialize)]
pub struct Donation
{
    pub amount : f32,
    pub fundraiser_name : String,
    #[serde(flatten)]
    payment : util::Payment,
}

#[derive(Deserialize,Debug, Serialize)]
pub struct Fundraiser {
    pub name : String,
    pub goal : f32,
    pub amount_raised : f32,
    pub formname : String,
    pub image : String,
    pub description : String,
}
//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
impl Fundraiser {
    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_button(&self) -> String {
        return format!(
            "<a href=\"{}\" class=\"w-50 btn btn-lg btn-success\" role=\"button\">Donate Now</a>",
            self.formname);
    }

    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_entry(&self) -> String {

          format!("<div class=\"col-md-6 col-lg-4 g-mb-30\"><article class=\"u-shadow-v18 g-bg-white text-center rounded g-px-20 g-py-40 g-mb-5\">
            <img class=\"d-inline-block img-fluid mb-4\" Width=\"400\" Height=\"200\" src=\"{}\" alt=\"Image Description\">
            <h4 class=\"h5 g-color-black g-font-weight-600 g-mb-10\">{}</h4>
            <p> {} </p>
            <div class=\"progress\">
              <div class=\"progress-bar bg-success\" role=\"progressbar\" style=\"width: {}%\" aria-valuenow=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"{}\">${} of ${} Raised</div>
            </div>
            <p>  </p>
            {}
            </article></div>",
          self.image,
          self.name,
          self.description,
          (100.0* (1.0- (self.goal - self.amount_raised)/self.goal)) as i32,
          self.amount_raised as i32,
          self.goal as i32,
          self.amount_raised as i32,
          self.goal as i32,
          self.get_button())
    }

}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn process_donation(
    donation : web::Form<Donation>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {

    let mut fundraisers = util::get_file::<BTreeMap<String, Fundraiser>>("fundraising_goals.json".to_string());

    debug!("fundraisers = {:#?}\n", fundraisers);
    let result = util::process_payment(&donation.payment, donation.amount, braintree, util::PaymentType::Donation, &donation.fundraiser_name);

    if result.is_err() {
        error!("Error: payment process {:#?}\n", result);
        return util::error(util::PaymentType::Donation).await;
    }

    info!("donation of {} processed for {}\n",donation.amount, donation.fundraiser_name);

    match fundraisers.get_mut(&donation.fundraiser_name) {
        Some(ref mut fundraiser) => {
            fundraiser.amount_raised += donation.amount;
            info!("amount_raised = {:#?}\n", fundraiser.amount_raised);
        },
        None => {
            error!("Error: unknown fundraiser name {}\n", donation.fundraiser_name);
            return util::error(util::PaymentType::Donation).await;
        },
    }

    debug!("fundraisers = {:?}\n", fundraisers);

    serde_json::to_writer(
        &File::create("fundraising_goals.json").expect("unable to open file"),
        &fundraisers).expect("unable to write fundraising_goals.json");

    util::thanks(util::PaymentType::Donation).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn fundraisers_page()-> HttpResponse {
    let web_page = include_str!("../static/fundraise.html");
    let fundraisers = util::get_file::<BTreeMap<String, Fundraiser>>("fundraising_goals.json".to_string());
    info!("fundraisers = {:?}\n", fundraisers);

    let mut content = String::new();
    for (_, fundraiser) in fundraisers.iter() {
        content += fundraiser.get_entry().as_str();
    }
    let web_page = web_page.replace("FUNDRAISERS", &content);

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(web_page)
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn fundraiser_page(braintree : web::Data<Mutex<Braintree>>, name : String) -> HttpResponse {
    let fundraisers = util::get_file::<BTreeMap<String, Fundraiser>>("fundraising_goals.json".to_string());
    let fundraiser = fundraisers.get(&name).unwrap();
    let braintree = braintree.lock().unwrap();

    info!("{} amount_raised = {}\n", name, fundraiser.amount_raised);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/donate.html")
              .replace("FORMNAME", &fundraiser.formname)
              .replace("NAME", &fundraiser.name)
              .replace("DESCRIPTION", &fundraiser.description)
              .replace(
                  "CLIENT_TOKEN_FROM_SERVER",
                  braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str()))
}

