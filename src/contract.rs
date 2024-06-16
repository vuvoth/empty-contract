use crate::{error::ContractError, msg::{ExecuteMsg, GreetResp, InstantiateMsg, QueryMsg}};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::state::ADMINS;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = msg
        .admins
        .into_iter()
        .map(|addr| deps.api.addr_validate(&addr))
        .collect();

    ADMINS.save(deps.storage, &admins?)?;

    Ok(Response::new())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Greet {} => to_json_binary(&query::greet()?),
        AdminsList {} => to_json_binary(&query::admins_list(deps)?),
    }
}

mod query {
    use crate::msg::AdminListResp;

    use super::*;

    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp {
            message: "Hello World".to_owned(),
        };

        Ok(resp)
    }

    pub fn admins_list(deps: Deps) -> StdResult<AdminListResp> {
        let admins = ADMINS.load(deps.storage)?;
        let resp = AdminListResp { admins };
        Ok(resp)
    }
}

#[allow(dead_code)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;
    match msg {
        AddMembers { admins } => exec::add_members(deps, info, admins),
        Leave {} => exec::leave(deps, info).map_err(Into::into),
    }
}

mod exec {
    use super::*;
    pub fn add_members(
        deps: DepsMut,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response, ContractError> {
        let mut curr_admins = ADMINS.load(deps.storage)?;
        if !curr_admins.contains(&info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender
            });
        }

        let admins: StdResult<Vec<_>> = admins
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();

        curr_admins.append(&mut admins?);

        ADMINS.save(deps.storage, &curr_admins)?;
        Ok(Response::new())
    }

    pub fn leave(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        ADMINS.update(deps.storage, move |admins| -> StdResult<_> {
            let admins = admins
                .into_iter()
                .filter(|admin| *admin != info.sender)
                .collect();
            Ok(admins)
        })?;

        Ok(Response::new())
    }
}
#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockApi, Addr};
    use cw_multi_test::{App, ContractWrapper, Executor};

    use crate::msg::AdminListResp;

    use super::*;

    #[test]
    fn instantiation() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let mock_api = MockApi::default();

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: AdminListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();
        assert_eq!(resp, AdminListResp { admins: vec![] });

        let admins: Vec<String> = ["admin1".to_owned(), "admin2".to_owned()]
            .iter()
            .map(|addr| mock_api.addr_make(addr).to_string())
            .collect();

        let admin_addresses: Vec<Addr> = admins.iter().map(Addr::unchecked).collect();

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins },
                &[],
                "Contract 2",
                None,
            )
            .unwrap();

        let resp: AdminListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();

        assert_eq!(
            resp,
            AdminListResp {
                admins: admin_addresses
            }
        );
    }

    #[test]
    fn greet_query() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);

        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let resp: GreetResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::Greet {})
            .unwrap();

        assert_eq!(
            resp,
            GreetResp {
                message: "Hello World".to_string()
            }
        );
    }

    #[test]
    fn execution() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();

        let admin = app.api().addr_make("user");
        let err = app.execute_contract(
            Addr::unchecked("user"), 
            addr, 
            &ExecuteMsg::AddMembers { admins: vec![admin.to_string()] },
            &[]
        ).unwrap_err();

        assert_eq!(
            ContractError::Unauthorized {
                sender: Addr::unchecked("user")
            },
            err.downcast().unwrap()
        );
    }
}
