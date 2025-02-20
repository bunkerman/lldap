use crate::{components::router::AppRoute, infra::api::HostService};
use anyhow::{bail, Result};
use graphql_client::GraphQLQuery;
use validator_derive::Validate;
use yew::prelude::*;
use yew::services::{fetch::FetchTask, ConsoleService};
use yew_form_derive::Model;
use yew_router::{
    agent::{RouteAgentDispatcher, RouteRequest},
    route::Route,
};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/create_group.graphql",
    response_derives = "Debug",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct CreateGroup;

pub struct CreateGroupForm {
    link: ComponentLink<Self>,
    route_dispatcher: RouteAgentDispatcher,
    form: yew_form::Form<CreateGroupModel>,
    error: Option<anyhow::Error>,
    // Used to keep the request alive long enough.
    task: Option<FetchTask>,
}

#[derive(Model, Validate, PartialEq, Clone, Default)]
pub struct CreateGroupModel {
    #[validate(length(min = 1, message = "Groupname is required"))]
    groupname: String,
}

pub enum Msg {
    Update,
    SubmitForm,
    CreateGroupResponse(Result<create_group::ResponseData>),
}

impl CreateGroupForm {
    fn handle_msg(&mut self, msg: <Self as Component>::Message) -> Result<bool> {
        match msg {
            Msg::Update => Ok(true),
            Msg::SubmitForm => {
                if !self.form.validate() {
                    bail!("Check the form for errors");
                }
                let model = self.form.model();
                let req = create_group::Variables {
                    name: model.groupname,
                };
                self.task = Some(HostService::graphql_query::<CreateGroup>(
                    req,
                    self.link.callback(Msg::CreateGroupResponse),
                    "Error trying to create group",
                )?);
                Ok(true)
            }
            Msg::CreateGroupResponse(response) => {
                ConsoleService::log(&format!(
                    "Created group '{}'",
                    &response?.create_group.display_name
                ));
                self.route_dispatcher
                    .send(RouteRequest::ChangeRoute(Route::from(AppRoute::ListGroups)));
                Ok(true)
            }
        }
    }
}

impl Component for CreateGroupForm {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            route_dispatcher: RouteAgentDispatcher::new(),
            form: yew_form::Form::<CreateGroupModel>::new(CreateGroupModel::default()),
            error: None,
            task: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.error = None;
        match self.handle_msg(msg) {
            Err(e) => {
                ConsoleService::error(&e.to_string());
                self.error = Some(e);
                self.task = None;
                true
            }
            Ok(b) => b,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        type Field = yew_form::Field<CreateGroupModel>;
        html! {
          <div class="row justify-content-center">
            <form class="form shadow-sm py-3" style="max-width: 636px">
              <div class="row mb-3">
                <h5 class="fw-bold">{"Create a group"}</h5>
              </div>
              <div class="form-group row mb-3">
                <label for="groupname"
                  class="form-label col-4 col-form-label">
                  {"Group name*:"}
                </label>
                <div class="col-8">
                  <Field
                    form=&self.form
                    field_name="groupname"
                    class="form-control"
                    class_invalid="is-invalid has-error"
                    class_valid="has-success"
                    autocomplete="groupname"
                    oninput=self.link.callback(|_| Msg::Update) />
                  <div class="invalid-feedback">
                    {&self.form.field_message("groupname")}
                  </div>
                </div>
              </div>
              <div class="form-group row justify-content-center">
                <button
                  class="btn btn-primary col-auto col-form-label"
                  type="submit"
                  disabled=self.task.is_some()
                  onclick=self.link.callback(|e: MouseEvent| {e.prevent_default(); Msg::SubmitForm})>
                  {"Submit"}
                </button>
              </div>
            </form>
            { if let Some(e) = &self.error {
                html! {
                  <div class="alert alert-danger">
                    {e.to_string() }
                  </div>
                }
              } else { html! {} }
            }
          </div>
        }
    }
}
