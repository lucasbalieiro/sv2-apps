use integration_tests_sv2::{interceptor::ReplaceMessage, template_provider::DifficultyLevel, *};
use interceptor::MessageDirection;
use stratum_apps::stratum_core::{
    common_messages_sv2::{
        SetupConnectionError, MESSAGE_TYPE_SETUP_CONNECTION, MESSAGE_TYPE_SETUP_CONNECTION_ERROR,
        MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
    },
    mining_sv2::{
        OpenMiningChannelError, MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL,
        MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCESS,
    },
    parsers_sv2::{self, AnyMessage},
};

// Demonstrates the scenario where TProxy falls back to the secondary pool
// after the primary pool returns a setup-connection error.
#[tokio::test]
async fn test_tp_fallback_on_setup_connection_error() {
    start_tracing();
    let (_tp, tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let (_pool_1, pool_addr_1) = start_pool(Some(tp_addr)).await;
    let (_pool_2, pool_addr_2) = start_pool(Some(tp_addr)).await;

    let random_error_code = "Something went wrong".to_string();

    let submit_solution_replace = ReplaceMessage::new(
        MessageDirection::ToDownstream,
        MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        AnyMessage::Common(parsers_sv2::CommonMessages::SetupConnectionError(
            SetupConnectionError {
                flags: 0,
                error_code: random_error_code.try_into().unwrap(),
            },
        )),
    );

    let (pool_translator_sniffer_1, pool_translator_sniffer_addr_1) = start_sniffer(
        "A",
        pool_addr_1,
        false,
        vec![submit_solution_replace.into()],
        None,
    );

    let (pool_translator_sniffer_2, pool_translator_sniffer_addr_2) =
        start_sniffer("B", pool_addr_2, false, vec![], None);

    let (_, _) = start_sv2_translator(
        &[
            pool_translator_sniffer_addr_1,
            pool_translator_sniffer_addr_2,
        ],
        false,
    )
    .await;

    pool_translator_sniffer_1
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;
    pool_translator_sniffer_1
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_ERROR,
        )
        .await;

    pool_translator_sniffer_2
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;

    pool_translator_sniffer_2
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;
}

// Demonstrates the scenario where the primary pool returns an OpenMiningChannelError,
// causing TProxy to fall back to the secondary pool.
#[tokio::test]
async fn test_tp_fallback_on_open_mining_message_error() {
    start_tracing();
    let (_tp, tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let (_pool_1, pool_addr_1) = start_pool(Some(tp_addr)).await;
    let (_pool_2, pool_addr_2) = start_pool(Some(tp_addr)).await;

    let random_error_code = "Something went wrong".to_string();

    let submit_solution_replace = ReplaceMessage::new(
        MessageDirection::ToDownstream,
        MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCESS,
        AnyMessage::Mining(parsers_sv2::Mining::OpenMiningChannelError(
            OpenMiningChannelError {
                request_id: 0,
                error_code: random_error_code.try_into().unwrap(),
            },
        )),
    );

    let (pool_translator_sniffer_1, pool_translator_sniffer_addr_1) = start_sniffer(
        "A",
        pool_addr_1,
        false,
        vec![submit_solution_replace.into()],
        None,
    );

    let (pool_translator_sniffer_2, pool_translator_sniffer_addr_2) =
        start_sniffer("B", pool_addr_2, false, vec![], None);

    let (_, tproxy_addr) = start_sv2_translator(
        &[
            pool_translator_sniffer_addr_1,
            pool_translator_sniffer_addr_2,
        ],
        false,
    )
    .await;

    let (_minerd_process, _minerd_addr) = start_minerd(tproxy_addr, None, None, false).await;

    pool_translator_sniffer_1
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;
    pool_translator_sniffer_1
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;
    pool_translator_sniffer_1
        .wait_for_message_type(
            MessageDirection::ToUpstream,
            MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL,
        )
        .await;

    pool_translator_sniffer_2
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;

    pool_translator_sniffer_2
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;
}
