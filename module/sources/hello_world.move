module hello_world::hello_world {
    use std::signer;
    use std::string::{Self, String};
    use aptos_framework::event;

    #[event]
    struct HelloEvent has drop, store {
        message: String,
        from: address,
    }

    struct GreetingCounter has key {
        count: u64,
    }

    public entry fun initialize(account: &signer) {
        let account_addr = signer::address_of(account);
        if (!exists<GreetingCounter>(account_addr)) {
            move_to(account, GreetingCounter { count: 0 });
        }
    }

    public entry fun say_hello(account: &signer) acquires GreetingCounter {
        let account_addr = signer::address_of(account);
        
        if (!exists<GreetingCounter>(account_addr)) {
            move_to(account, GreetingCounter { count: 0 });
        };

        let counter = borrow_global_mut<GreetingCounter>(account_addr);
        counter.count = counter.count + 1;

        event::emit(HelloEvent {
            message: string::utf8(b"Hello, World!"),
            from: account_addr,
        });
    }

    public fun get_greeting_count(account_addr: address): u64 acquires GreetingCounter {
        if (exists<GreetingCounter>(account_addr)) {
            borrow_global<GreetingCounter>(account_addr).count
        } else {
            0
        }
    }

    public fun has_greeted(account_addr: address): bool {
        exists<GreetingCounter>(account_addr)
    }
}