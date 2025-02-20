use crate::{
    components::{
        add_group_member::{self, AddGroupMemberComponent},
        remove_user_from_group::RemoveUserFromGroupComponent,
        router::{AppRoute, Link},
    },
    infra::api::HostService,
};
use anyhow::{bail, Error, Result};
use graphql_client::GraphQLQuery;
use yew::{
    prelude::*,
    services::{fetch::FetchTask, ConsoleService},
};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../schema.graphql",
    query_path = "queries/get_group_details.graphql",
    response_derives = "Debug, Hash, PartialEq, Eq, Clone",
    custom_scalars_module = "crate::infra::graphql"
)]
pub struct GetGroupDetails;

pub type Group = get_group_details::GetGroupDetailsGroup;
pub type User = get_group_details::GetGroupDetailsGroupUsers;
pub type AddGroupMemberUser = add_group_member::User;

pub struct GroupDetails {
    link: ComponentLink<Self>,
    props: Props,
    /// The group info. If none, the error is in `error`. If `error` is None, then we haven't
    /// received the server response yet.
    group: Option<Group>,
    /// Error message displayed to the user.
    error: Option<Error>,
    // Used to keep the request alive long enough.
    _task: Option<FetchTask>,
}

/// State machine describing the possible transitions of the component state.
/// It starts out by fetching the user's details from the backend when loading.
pub enum Msg {
    /// Received the group details response, either the group data or an error.
    GroupDetailsResponse(Result<get_group_details::ResponseData>),
    OnError(Error),
    OnUserAddedToGroup(AddGroupMemberUser),
    OnUserRemovedFromGroup((String, i64)),
}

#[derive(yew::Properties, Clone, PartialEq)]
pub struct Props {
    pub group_id: i64,
}

impl GroupDetails {
    fn get_group_details(&mut self) {
        self._task = HostService::graphql_query::<GetGroupDetails>(
            get_group_details::Variables {
                id: self.props.group_id,
            },
            self.link.callback(Msg::GroupDetailsResponse),
            "Error trying to fetch group details",
        )
        .map_err(|e| {
            ConsoleService::log(&e.to_string());
            e
        })
        .ok();
    }

    fn handle_msg(&mut self, msg: <Self as Component>::Message) -> Result<bool> {
        match msg {
            Msg::GroupDetailsResponse(response) => match response {
                Ok(group) => self.group = Some(group.group),
                Err(e) => {
                    self.group = None;
                    bail!("Error getting user details: {}", e);
                }
            },
            Msg::OnError(e) => return Err(e),
            Msg::OnUserAddedToGroup(user) => {
                self.group.as_mut().unwrap().users.push(User {
                    id: user.id,
                    display_name: user.display_name,
                });
            }
            Msg::OnUserRemovedFromGroup((user_id, _)) => {
                self.group
                    .as_mut()
                    .unwrap()
                    .users
                    .retain(|u| u.id != user_id);
            }
        }
        Ok(true)
    }

    fn view_messages(&self, error: &Option<Error>) -> Html {
        if let Some(e) = error {
            html! {
              <div class="alert alert-danger">
                <span>{"Error: "}{e.to_string()}</span>
              </div>
            }
        } else {
            html! {}
        }
    }

    fn view_user_list(&self, g: &Group) -> Html {
        let make_user_row = |user: &User| {
            let user_id = user.id.clone();
            let display_name = user.display_name.clone();
            html! {
              <tr>
                <td>
                  <Link route=AppRoute::UserDetails(user_id.clone())>
                    {user_id.clone()}
                  </Link>
                </td>
                <td>{display_name}</td>
                <td>
                  <RemoveUserFromGroupComponent
                    username=user_id
                    group_id=g.id
                    on_user_removed_from_group=self.link.callback(Msg::OnUserRemovedFromGroup)
                    on_error=self.link.callback(Msg::OnError)/>
                </td>
              </tr>
            }
        };
        html! {
          <>
            <h3>{g.display_name.to_string()}</h3>
            <h5 class="fw-bold">{"Members"}</h5>
            <div class="table-responsive">
              <table class="table table-striped">
                <thead>
                  <tr key="headerRow">
                    <th>{"User Id"}</th>
                    <th>{"Display name"}</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {if g.users.is_empty() {
                    html! {
                      <tr key="EmptyRow">
                        <td>{"No members"}</td>
                        <td/>
                      </tr>
                    }
                  } else {
                    html! {<>{g.users.iter().map(make_user_row).collect::<Vec<_>>()}</>}
                  }}
                </tbody>
              </table>
            </div>
          </>
        }
    }

    fn view_add_user_button(&self, g: &Group) -> Html {
        let users: Vec<_> = g
            .users
            .iter()
            .map(|u| AddGroupMemberUser {
                id: u.id.clone(),
                display_name: u.display_name.clone(),
            })
            .collect();
        html! {
            <AddGroupMemberComponent
                group_id=g.id
                users=users
                on_error=self.link.callback(Msg::OnError)
                on_user_added_to_group=self.link.callback(Msg::OnUserAddedToGroup)/>
        }
    }
}

impl Component for GroupDetails {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut table = Self {
            link,
            props,
            _task: None,
            group: None,
            error: None,
        };
        table.get_group_details();
        table
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.error = None;
        match self.handle_msg(msg) {
            Err(e) => {
                ConsoleService::error(&e.to_string());
                self.error = Some(e);
                true
            }
            Ok(b) => b,
        }
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        match (&self.group, &self.error) {
            (None, None) => html! {{"Loading..."}},
            (None, Some(e)) => html! {<div>{"Error: "}{e.to_string()}</div>},
            (Some(u), error) => {
                html! {
                    <div>
                      {self.view_user_list(u)}
                      {self.view_add_user_button(u)}
                      {self.view_messages(error)}
                    </div>
                }
            }
        }
    }
}
