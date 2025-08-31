[private]
default:
	@just --list --unsorted

test:
    cargo test
    cd tests && just

test-assets:
    cd test-assets && just
