use crate::{
    components::select::{Select, SelectOption, SelectOptionProps},
    infra::api::HostService,
};
use anyhow::{Error, Result};
use graphql_client::GraphQLQuery;
use std::collections::HashSet;
use yew::{
    prelude::*,
    services::{fetch::FetchTask, ConsoleService},
};
use yewtil::NeqAssign;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/add_user_to_group.graphql",
    response_derives = "Debug",
    variables_derives = "Clone",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct AddUserToGroup;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/list_users.graphql",
    response_derives = "Debug,Clone,PartialEq,Eq,Hash",
    variables_derives = "Clone",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct ListUserNames;
pub type User = list_user_names::ListUserNamesUsers;

pub struct AddGroupMemberComponent {
    link: ComponentLink<Self>,
    props: Props,
    /// The list of existing users, initially not loaded.
    user_list: Option<Vec<User>>,
    /// The currently selected user.
    selected_user: Option<User>,
    // Used to keep the request alive long enough.
    task: Option<FetchTask>,
}

pub enum Msg {
    UserListResponse(Result<list_user_names::ResponseData>),
    SubmitAddMember,
    AddMemberResponse(Result<add_user_to_group::ResponseData>),
    SelectionChanged(Option<SelectOptionProps>),
}

#[derive(yew::Properties, Clone, PartialEq)]
pub struct Props {
    pub group_id: i64,
    pub users: Vec<User>,
    pub on_user_added_to_group: Callback<User>,
    pub on_error: Callback<Error>,
}

impl AddGroupMemberComponent {
    fn get_user_list(&mut self) {
        self.task = HostService::graphql_query::<ListUserNames>(
            list_user_names::Variables { filters: None },
            self.link.callback(Msg::UserListResponse),
            "Error trying to fetch user list",
        )
        .map_err(|e| {
            ConsoleService::log(&e.to_string());
            e
        })
        .ok();
    }

    fn submit_add_member(&mut self) -> Result<bool> {
        let user_id = match self.selected_user.clone() {
            None => return Ok(false),
            Some(user) => user.id,
        };
        self.task = HostService::graphql_query::<AddUserToGroup>(
            add_user_to_group::Variables {
                user: user_id,
                group: self.props.group_id,
            },
            self.link.callback(Msg::AddMemberResponse),
            "Error trying to initiate adding the user to a group",
        )
        .map_err(|e| {
            ConsoleService::log(&e.to_string());
            e
        })
        .ok();
        Ok(true)
    }

    fn handle_msg(&mut self, msg: <Self as Component>::Message) -> Result<bool> {
        match msg {
            Msg::UserListResponse(response) => {
                self.user_list = Some(response?.users);
                self.task = None;
            }
            Msg::SubmitAddMember => return self.submit_add_member(),
            Msg::AddMemberResponse(response) => {
                response?;
                self.task = None;
                let user = self
                    .selected_user
                    .as_ref()
                    .expect("Could not get selected user")
                    .clone();
                // Remove the user from the dropdown.
                self.props.on_user_added_to_group.emit(user);
            }
            Msg::SelectionChanged(option_props) => {
                let was_some = self.selected_user.is_some();
                self.selected_user = option_props.map(|u| User {
                    id: u.value,
                    display_name: u.text,
                });
                return Ok(self.selected_user.is_some() != was_some);
            }
        }
        Ok(true)
    }

    fn get_selectable_user_list(&self, user_list: &[User]) -> Vec<User> {
        let user_groups = self.props.users.iter().collect::<HashSet<_>>();
        user_list
            .iter()
            .filter(|u| !user_groups.contains(u))
            .map(Clone::clone)
            .collect()
    }
}

impl Component for AddGroupMemberComponent {
    type Message = Msg;
    type Properties = Props;
    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut res = Self {
            link,
            props,
            user_list: None,
            selected_user: None,
            task: None,
        };
        res.get_user_list();
        res
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match self.handle_msg(msg) {
            Err(e) => {
                ConsoleService::error(&e.to_string());
                self.props.on_error.emit(e);
                self.task = None;
                true
            }
            Ok(b) => b,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        if let Some(user_list) = &self.user_list {
            let to_add_user_list = self.get_selectable_user_list(user_list);
            #[allow(unused_braces)]
            let make_select_option = |user: User| {
                html_nested! {
                    <SelectOption value=user.id.clone() text=user.display_name.clone() key=user.id />
                }
            };
            html! {
            <div class="row">
              <div class="col-sm-3">
                <Select on_selection_change=self.link.callback(Msg::SelectionChanged)>
                  {
                    to_add_user_list
                        .into_iter()
                        .map(make_select_option)
                        .collect::<Vec<_>>()
                  }
                </Select>
              </div>
              <div class="col-sm-1">
                <button
                  class="btn btn-success"
                  disabled=self.selected_user.is_none() || self.task.is_some()
                  onclick=self.link.callback(|_| Msg::SubmitAddMember)>
                  {"Add"}
                </button>
              </div>
            </div>
            }
        } else {
            html! {
              {"Loading groups"}
            }
        }
    }
}
