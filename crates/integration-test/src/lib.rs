//! Test-support helpers for the kind trees integration tests.

use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy_chains::NamedChain;
use anoma_pa_evm_bindings::helpers::alchemy_url;
use anoma_risc0_kind_trees::{Caip2ChainId, SolanaAddress, SolanaCluster};
use anyhow::{Context, Result, ensure};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

/// Connects an Alchemy-backed provider for the chain. Every chain carrying supported tokens or a recorded
/// deployment must be reachable this way; an unknown chain, or a chain that is not an EVM chain, is a hard failure,
/// never a skip.
pub fn provider(chain: &Caip2ChainId) -> Result<DynProvider> {
    let evm_chain = NamedChain::try_from(chain)?;
    let url = alchemy_url(&evm_chain).with_context(|| format!("no RPC route for {chain}"))?;
    Ok(ProviderBuilder::new().connect_http(url).erased())
}

/// Connects an RPC client to the Solana cluster: `SOLANA_RPC_URL_DEVNET` / `SOLANA_RPC_URL_MAINNET_BETA` when set,
/// else the cluster's public endpoint.
pub fn solana_rpc(cluster: SolanaCluster) -> RpcClient {
    let (variable, public) = match cluster {
        SolanaCluster::Devnet => ("SOLANA_RPC_URL_DEVNET", "https://api.devnet.solana.com"),
        SolanaCluster::MainnetBeta => (
            "SOLANA_RPC_URL_MAINNET_BETA",
            "https://api.mainnet-beta.solana.com",
        ),
    };
    RpcClient::new(std::env::var(variable).unwrap_or_else(|_| public.to_string()))
}

/// The decimals of the SPL token mint at the address, or `None` when no account is there. An account that exists
/// must be owned by the SPL Token program.
pub async fn mint_decimals(rpc: &RpcClient, mint: &SolanaAddress) -> Result<Option<u8>> {
    let mint = Pubkey::new_from_array(*mint.as_bytes());
    let Some(account) = rpc
        .get_account_with_commitment(&mint, rpc.commitment())
        .await
        .with_context(|| format!("reading {mint}"))?
        .value
    else {
        return Ok(None);
    };
    ensure!(
        account.owner == spl_token_interface::ID,
        "{mint}: owned by {}, not the SPL Token program {}",
        account.owner,
        spl_token_interface::ID
    );
    let decoded = spl_token_interface::state::Mint::unpack(&account.data)
        .with_context(|| format!("{mint}: not an SPL token mint"))?;
    Ok(Some(decoded.decimals))
}
