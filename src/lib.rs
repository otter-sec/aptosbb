use anyhow::Result;
use aptos_language_e2e_tests::{
    account::Account,
    executor::FakeExecutor,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{TransactionPayload, TransactionStatus, EntryFunction},
    account_config::AccountResource,
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_cached_packages::aptos_stdlib;
use aptos_rest_client::{Client, AptosBaseUrl};
use std::{path::Path, collections::HashMap};

pub mod pentest;

/// Main interface for the AptosBB pentesting environment
pub struct AptosBB {
    executor: FakeExecutor,
    sequence_numbers: HashMap<AccountAddress, u64>,
    chain_id: aptos_types::chain_id::ChainId,
}

impl AptosBB {
    /// Create AptosBB with remote mainnet state at the latest version
    pub async fn from_mainnet_latest() -> Result<Self> {
        let base_url = AptosBaseUrl::Mainnet;
        let client = Client::new(base_url.to_url().clone());
        let ledger_info = client.get_ledger_information().await?
            .into_inner();
        let latest_version = ledger_info.version;
        
        println!("Connecting to mainnet at latest version: {}", latest_version);
        println!("Chain ID: {}", ledger_info.chain_id);
        
        let mut executor = FakeExecutor::from_remote_state(base_url, latest_version);
        
        let timestamp_secs = ledger_info.timestamp_usecs / 1_000_000;
        executor.set_block_time(timestamp_secs);
        println!("Set executor block time to: {}", timestamp_secs);
        
        Ok(Self {
            executor,
            sequence_numbers: HashMap::new(),
            chain_id: aptos_types::chain_id::ChainId::new(ledger_info.chain_id),
        })
    }
    
    /// Create AptosBB with remote mainnet state at the latest version (with API key)
    pub async fn from_mainnet_latest_with_api_key(api_key: &str) -> Result<Self> {
        let base_url = AptosBaseUrl::Mainnet;
        
        let client = Client::new(base_url.to_url().clone());
        
        let ledger_info = client.get_ledger_information().await?
            .into_inner();
        let latest_version = ledger_info.version;
        
        println!("Connecting to mainnet at latest version: {} (with API key)", latest_version);
        println!("Chain ID: {}", ledger_info.chain_id);
        
        let mut executor = FakeExecutor::from_remote_state_with_api_key(base_url, latest_version, api_key);
        
        let timestamp_secs = ledger_info.timestamp_usecs / 1_000_000;
        executor.set_block_time(timestamp_secs);
        println!("Set executor block time to: {}", timestamp_secs);
        
        Ok(Self {
            executor,
            sequence_numbers: HashMap::new(),
            chain_id: aptos_types::chain_id::ChainId::new(ledger_info.chain_id),
        })
    }
    
    /// Create a new account with balance
    pub fn new_account(&mut self) -> Account {
        let account = Account::new();
        let executor_account = self.executor.new_account_at(*account.address());
        self.sequence_numbers.insert(*executor_account.address(), 0);
        if let Some(account_resource) = self.read_account_resource_at_address(executor_account.address()) {
            println!("Account created at address: {}", executor_account.address());
            println!("   Sequence number: {}", account_resource.sequence_number());
        }
        
        executor_account
    }
    
    /// Create an account at a specific address
    pub fn new_account_at(&mut self, addr: AccountAddress) -> Account {
        let account = self.executor.new_account_at(addr);
        
        if let Some(_account_resource) = self.read_account_resource_at_address(&addr) {
            self.sequence_numbers.insert(addr, 0);
            account
        } else {
            eprintln!("Warning: Account creation at address {} may have failed - AccountResource not found", addr);
            account
        }
    }
    
    /// Publish a Move package
    pub fn publish_package(&mut self, account: &Account, path: &Path) -> TransactionStatus {
        let build_options = BuildOptions {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            ..BuildOptions::default()
        };
        
        let package = match BuiltPackage::build(path.to_path_buf(), build_options) {
            Ok(pkg) => pkg,
            Err(e) => {
                eprintln!("Failed to build package: {}", e);
                use aptos_types::transaction::ExecutionStatus;
                return TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                    aptos_types::vm_status::StatusCode::ABORTED.into()
                )));
            }
        };
        
        let payload = self.generate_module_payload(&package);
        self.run_transaction(account, payload)
    }
    
    /// Generate a TransactionPayload for publishing modules
    fn generate_module_payload(&self, package: &BuiltPackage) -> TransactionPayload {
        let code = package.extract_code();
        let metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        )
    }
    
    /// Run an entry function
    pub fn run_entry_function(
        &mut self,
        account: &Account,
        module: AccountAddress,
        module_name: &str,
        function: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> TransactionStatus {
        
        let module_id = ModuleId::new(
            module,
            Identifier::new(module_name).unwrap()
        );
        
        let payload = TransactionPayload::EntryFunction(EntryFunction::new(
            module_id,
            Identifier::new(function).unwrap(),
            ty_args,
            args,
        ));
        
        self.run_transaction(account, payload)
    }
    
    /// Run transaction with custom payload and return full output
    pub fn run_transaction_with_output(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> (TransactionStatus, aptos_types::transaction::TransactionOutput) {
        let sequence_number = *self.sequence_numbers.get(account.address()).unwrap_or(&0);
        self.sequence_numbers.insert(*account.address(), sequence_number + 1);
        
        // Use a longer TTL to avoid expiration
        let ttl = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 300; // 5 minutes from now
            
        let txn = account
            .transaction()
            .payload(payload)
            .sequence_number(sequence_number)
            .max_gas_amount(2_000_000)
            .gas_unit_price(100)
            .ttl(ttl)
            .chain_id(self.chain_id)
            .sign();
        
        let output = self.executor.execute_and_apply(txn);
        let status = output.status().to_owned();
        
        (status, output)
    }
    
    /// Run transaction with custom payload
    pub fn run_transaction(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> TransactionStatus {
        let (status, _) = self.run_transaction_with_output(account, payload);
        status
    }
    
    /// Read a resource from an address
    pub fn read_resource<T>(&self, addr: &AccountAddress) -> Option<T> 
    where 
        T: serde::de::DeserializeOwned + move_core_types::move_resource::MoveResource,
    {
        self.executor.read_resource::<T>(addr)
    }
    
    /// Check if a resource exists
    pub fn exists_resource(
        &self,
        addr: &AccountAddress,
        struct_tag: move_core_types::language_storage::StructTag,
    ) -> bool {
        use aptos_types::access_path::AccessPath;
        use aptos_types::state_store::state_key::StateKey;
        
        if let Ok(state_key) = StateKey::resource(addr, &struct_tag) {
            if self.executor.read_state_value(&state_key).is_some() {
                return true;
            }
        }
        
        if let Ok(path) = AccessPath::resource_access_path(*addr, struct_tag) {
            let key_bytes = bcs::to_bytes(&path).unwrap();
            if let Ok(key) = StateKey::decode(&key_bytes) {
                return self.executor.read_state_value(&key).is_some();
            }
        }
        
        false
    }
    
    /// Execute a view function
    pub fn execute_view_function(
        &mut self,
        module: AccountAddress,
        module_name: &str,
        function: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>> {
        use aptos_types::move_utils::MemberId;
        
        let function_id = format!("{}::{}::{}", module, module_name, function);
        let member_id: MemberId = function_id.parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse function ID: {}", e))?;
        
        let output = self.executor.execute_view_function(member_id, ty_args, args);
        
        match output.values {
            Ok(results) => Ok(results),
            Err(e) => Err(anyhow::anyhow!("View function execution failed: {:?}", e)),
        }
    }
    
    /// Reads the resource `Value` for an account under the given address from
    /// this executor's data store.
    pub fn read_account_resource_at_address(
        &self,
        addr: &AccountAddress,
    ) -> Option<AccountResource> {
        self.read_resource(addr)
    }
    
    /// Verify that an account exists and is properly initialized
    pub fn verify_account_exists(&self, address: &AccountAddress) -> bool {
        self.read_account_resource_at_address(address).is_some()
    }
    
    /// Read APT balance using both CoinStore and FungibleStore
    pub fn read_aptos_balance(&self, addr: &AccountAddress) -> u64 {
        if let Some(balance) = self.read_apt_fungible_store_internal(addr) {
            return balance;
        }
        
        0
    }
    
    /// Internal helper to read from fungible store
    fn read_apt_fungible_store_internal(&self, addr: &AccountAddress) -> Option<u64> {
        use aptos_types::account_config::fungible_store::{FungibleStoreResource, primary_apt_store};
        use aptos_types::account_config::ObjectGroupResource;
        
        self.executor
            .read_resource_from_group::<FungibleStoreResource>(
                &primary_apt_store(*addr),
                &ObjectGroupResource::struct_tag(),
            )
            .map(|c| c.balance())
    }
    
    /// Reads the APT FungibleStore resource value for an account from this executor's data store.
    pub fn read_apt_fungible_store_resource(&self, account: &Account) -> Option<u64> {
        use aptos_types::account_config::fungible_store::{FungibleStoreResource, primary_apt_store};
        use aptos_types::account_config::ObjectGroupResource;
        
        self.executor
            .read_resource_from_group::<FungibleStoreResource>(
                &primary_apt_store(*account.address()),
                &ObjectGroupResource::struct_tag(),
            )
            .map(|c| c.balance())
    }
 
    /// Get APT balance for an account 
    pub fn get_apt_balance(&self, address: &AccountAddress) -> Option<u64> {
        let balance = self.read_aptos_balance(address);
        if balance > 0 {
            Some(balance)
        } else {
            None
        }
    }
    
    /// Check if an account has APT balance (any method available)
    pub fn has_apt_balance(&self, account: &Account) -> bool {
        self.read_aptos_balance(account.address()) > 0
    }
    
    /// Read raw state value at a state key
    pub fn read_state_value(&self, state_key: &aptos_types::state_store::state_key::StateKey) -> Option<aptos_types::state_store::state_value::StateValue> {
        self.executor.read_state_value(state_key)
    }
    
}