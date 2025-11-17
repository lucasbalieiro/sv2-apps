use crate::error::TemplateDataError;

use bitcoin_capnp_types::{
    mining_capnp::block_template::Client as BlockTemplateIpcClient,
    proxy_capnp::thread::Client as ThreadIpcClient,
};
use stratum_core::bitcoin::{
    Target, Transaction, TxOut,
    amount::{Amount, CheckedSum},
    block::{Block, Header, Version},
    consensus::{deserialize, serialize},
    hashes::{Hash, HashEngine, sha256d},
};

use stratum_core::{
    binary_sv2::{B016M, B064K, B0255, Seq064K, Seq0255, U256},
    template_distribution_sv2::{
        NewTemplate, RequestTransactionDataSuccess, SetNewPrevHash, SubmitSolution,
    },
};

#[derive(Clone)]
pub struct TemplateData {
    template_id: u64,
    header: Header,
    coinbase_tx: Transaction,
    merkle_path: Vec<Vec<u8>>,
    template_ipc_client: BlockTemplateIpcClient,
}

// impl block for public methods
impl TemplateData {
    pub fn new(
        template_id: u64,
        header: Header,
        coinbase_tx: Transaction,
        merkle_path: Vec<Vec<u8>>,
        template_ipc_client: BlockTemplateIpcClient,
    ) -> Self {
        Self {
            template_id,
            header,
            coinbase_tx,
            merkle_path,
            template_ipc_client,
        }
    }

    /// Destroys the template IPC client, cleaning up the resources on the Bitcoin Core side
    pub async fn destroy_ipc_client(
        &self,
        thread_ipc_client: ThreadIpcClient,
    ) -> Result<(), TemplateDataError> {
        tracing::debug!("Destroying template IPC client: {}", self.template_id);
        let mut destroy_ipc_client_request = self.template_ipc_client.destroy_request();
        let destroy_ipc_client_request_params = destroy_ipc_client_request.get();

        destroy_ipc_client_request_params
            .get_context()?
            .set_thread(thread_ipc_client);

        destroy_ipc_client_request.send().promise.await?;

        Ok(())
    }

    pub fn get_template_id(&self) -> u64 {
        self.template_id
    }

    pub fn get_new_template_message(
        &self,
        future_template: bool,
    ) -> Result<NewTemplate<'static>, TemplateDataError> {
        let new_template = NewTemplate {
            template_id: self.template_id,
            future_template,
            version: self.get_version()?,
            coinbase_tx_version: self.get_coinbase_tx_version()?,
            coinbase_prefix: self.get_coinbase_script_sig()?,
            coinbase_tx_input_sequence: self.get_coinbase_input_sequence(),
            coinbase_tx_value_remaining: self.get_coinbase_tx_value_remaining()?,
            coinbase_tx_outputs_count: self.get_empty_coinbase_outputs().len() as u32,
            coinbase_tx_outputs: self.get_serialized_empty_coinbase_outputs()?,
            coinbase_tx_locktime: self.get_coinbase_tx_lock_time(),
            merkle_path: self.get_merkle_path()?,
        };
        Ok(new_template.into_static())
    }

    // please note that `SetNewPrevHash.target` is consensus and not weak-block
    // so it's essentially redundant with `SetNewPrevHash.n_bits`
    pub fn get_set_new_prev_hash_message(&self) -> SetNewPrevHash<'static> {
        let set_new_prev_hash = SetNewPrevHash {
            template_id: self.template_id,
            prev_hash: self.get_prev_hash(),
            header_timestamp: self.get_ntime(),
            n_bits: self.get_nbits(),
            target: self.get_target(),
        };
        set_new_prev_hash.into_static()
    }

    pub async fn get_request_transaction_data_success_message(
        &self,
        thread_ipc_client: ThreadIpcClient,
    ) -> Result<RequestTransactionDataSuccess<'static>, TemplateDataError> {
        let request_transaction_data_success = RequestTransactionDataSuccess {
            template_id: self.template_id,
            transaction_list: self.get_tx_data(thread_ipc_client).await?,
            excess_data: vec![]
                .try_into()
                .expect("empty vec should always be valid for B064K"),
        };
        Ok(request_transaction_data_success.into_static())
    }

    pub fn get_prev_hash(&self) -> U256<'static> {
        self.header.prev_blockhash.to_byte_array().into()
    }

    pub async fn submit_solution(
        &self,
        submit_solution: SubmitSolution<'static>,
        thread_ipc_client: ThreadIpcClient,
    ) -> Result<(), TemplateDataError> {
        let solution_coinbase_tx_bytes: Vec<u8> = submit_solution.coinbase_tx.to_vec();

        let solution_coinbase_tx: Transaction =
            deserialize(&solution_coinbase_tx_bytes).map_err(|e| {
                tracing::error!("SubmitSolution.coinbase_tx is invalid: {}", e);
                TemplateDataError::InvalidCoinbaseTx(e)
            })?;

        // validate that the solution coinbase tx is congruent with the original coinbase tx
        if solution_coinbase_tx.version != self.coinbase_tx.version {
            return Err(TemplateDataError::InvalidCoinbaseTxVersion);
        }
        if solution_coinbase_tx.lock_time != self.coinbase_tx.lock_time {
            return Err(TemplateDataError::InvalidSolution);
        }
        if solution_coinbase_tx.input.len() != 1 {
            return Err(TemplateDataError::InvalidSolution);
        }
        if solution_coinbase_tx.input[0].sequence != self.coinbase_tx.input[0].sequence {
            return Err(TemplateDataError::InvalidSolution);
        }
        if solution_coinbase_tx.input[0].witness != self.coinbase_tx.input[0].witness {
            return Err(TemplateDataError::InvalidSolution);
        }
        if solution_coinbase_tx.input[0].previous_output
            != self.coinbase_tx.input[0].previous_output
        {
            return Err(TemplateDataError::InvalidSolution);
        }

        // Compute merkle root from coinbase transaction and merkle path
        let coinbase_txid = solution_coinbase_tx.compute_txid();
        let mut current_hash = *coinbase_txid.as_byte_array();

        // Combine with each sibling hash in the merkle path
        for sibling_hash_bytes in &self.merkle_path {
            if sibling_hash_bytes.len() != 32 {
                return Err(TemplateDataError::FailedToConvertMerklePathHashToU256);
            }

            // Combine current hash with sibling hash and double SHA256
            let mut hasher = sha256d::Hash::engine();
            HashEngine::input(&mut hasher, &current_hash);
            HashEngine::input(&mut hasher, sibling_hash_bytes);
            current_hash = *sha256d::Hash::from_engine(hasher).as_byte_array();
        }

        let solution_header = Header {
            version: Version::from_consensus(submit_solution.version as i32),
            prev_blockhash: self.header.prev_blockhash,
            merkle_root: sha256d::Hash::from_byte_array(current_hash).into(),
            time: submit_solution.header_timestamp,
            nonce: submit_solution.header_nonce,
            bits: self.header.bits,
        };

        solution_header
            .validate_pow(solution_header.target())
            .map_err(|e| {
                tracing::error!("SubmitSolution solution header is invalid: {}", e);
                TemplateDataError::InvalidSolutionPoW(e)
            })?;

        let mut submit_solution_request = self.template_ipc_client.submit_solution_request();
        let mut submit_solution_request_params = submit_solution_request.get();

        submit_solution_request_params.set_version(submit_solution.version);
        submit_solution_request_params.set_timestamp(submit_solution.header_timestamp);
        submit_solution_request_params.set_nonce(submit_solution.header_nonce);
        submit_solution_request_params.set_coinbase(&solution_coinbase_tx_bytes);

        submit_solution_request_params
            .get_context()?
            .set_thread(thread_ipc_client.clone());

        let submit_solution_response = submit_solution_request.send().promise.await?;

        if !submit_solution_response.get()?.get_result() {
            return Err(TemplateDataError::FailedIpcSubmitSolution);
        }

        Ok(())
    }
}

// impl block for private methods
impl TemplateData {
    fn get_nbits(&self) -> u32 {
        self.header.bits.to_consensus()
    }

    fn get_target(&self) -> U256<'_> {
        let target = Target::from(self.header.bits);
        let target_bytes: [u8; 32] = target.to_le_bytes();
        U256::from(target_bytes)
    }

    fn get_ntime(&self) -> u32 {
        self.header.time
    }

    fn get_version(&self) -> Result<u32, TemplateDataError> {
        self.header
            .version
            .to_consensus()
            .try_into()
            .map_err(|_| TemplateDataError::InvalidBlockVersion)
    }

    fn get_coinbase_tx_version(&self) -> Result<u32, TemplateDataError> {
        self.coinbase_tx
            .version
            .0
            .try_into()
            .map_err(|_| TemplateDataError::InvalidCoinbaseTxVersion)
    }

    fn get_coinbase_script_sig(&self) -> Result<B0255<'_>, TemplateDataError> {
        let coinbase_script_sig: B0255 = self.coinbase_tx.input[0]
            .script_sig
            .to_bytes()
            .try_into()
            .map_err(|_| TemplateDataError::InvalidCoinbaseScriptSig)?;
        Ok(coinbase_script_sig)
    }

    fn get_coinbase_input_sequence(&self) -> u32 {
        self.coinbase_tx.input[0].sequence.to_consensus_u32()
    }

    fn get_empty_coinbase_outputs(&self) -> Vec<TxOut> {
        self.coinbase_tx
            .output
            .iter()
            .filter(|output| output.value == Amount::from_sat(0))
            .cloned()
            .collect()
    }

    fn get_serialized_empty_coinbase_outputs(&self) -> Result<B064K<'_>, TemplateDataError> {
        let empty_coinbase_outputs = self.get_empty_coinbase_outputs();
        let mut serialized_empty_coinbase_outputs = Vec::new();
        for output in empty_coinbase_outputs {
            serialized_empty_coinbase_outputs.extend_from_slice(&serialize(&output));
        }
        let serialized_empty_coinbase_outputs: B064K = serialized_empty_coinbase_outputs
            .try_into()
            .map_err(|_| TemplateDataError::FailedToSerializeEmptyCoinbaseOutputs)?;
        Ok(serialized_empty_coinbase_outputs)
    }

    fn get_coinbase_tx_value_remaining(&self) -> Result<u64, TemplateDataError> {
        Ok(self
            .coinbase_tx
            .output
            .iter()
            .map(|output| output.value)
            .checked_sum()
            .ok_or(TemplateDataError::FailedToSumCoinbaseOutputs)?
            .to_sat())
    }

    fn get_coinbase_tx_lock_time(&self) -> u32 {
        self.coinbase_tx.lock_time.to_consensus_u32()
    }

    async fn get_tx_data(
        &self,
        thread_ipc_client: ThreadIpcClient,
    ) -> Result<Seq064K<'_, B016M<'static>>, TemplateDataError> {
        let mut template_block_request = self.template_ipc_client.get_block_request();
        template_block_request
            .get()
            .get_context()?
            .set_thread(thread_ipc_client.clone());

        let template_block_response = template_block_request.send().promise.await?;
        let template_block_bytes = template_block_response.get()?.get_result()?;

        // Deserialize the complete block template from Bitcoin Core's serialization format
        tracing::debug!(
            "Deserializing block template ({} bytes)",
            template_block_bytes.len()
        );
        let block: Block = deserialize(template_block_bytes)?;
        tracing::debug!(
            "Block deserialized - prev_hash from header: {:?}",
            block.header.prev_blockhash
        );

        let tx_data: Vec<B016M<'static>> = block
            .txdata
            .iter()
            .map(|tx| {
                serialize(tx)
                    .try_into()
                    .expect("tx data should always be valid for B016M")
            })
            .collect();
        Ok(Seq064K::new(tx_data).expect("tx data should always be valid for Seq064K"))
    }

    fn get_merkle_path(&self) -> Result<Seq0255<'_, U256<'_>>, TemplateDataError> {
        // Convert each Vec<u8> in the merkle path to U256
        let merkle_path_u256: Vec<U256<'_>> = self
            .merkle_path
            .iter()
            .map(|hash_bytes| {
                // Convert Vec<u8> to U256
                U256::try_from(hash_bytes.clone())
                    .map_err(|_| TemplateDataError::FailedToConvertMerklePathHashToU256)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Seq0255::new(merkle_path_u256).map_err(|_| TemplateDataError::FailedToCreateMerklePathSeq)
    }
}
