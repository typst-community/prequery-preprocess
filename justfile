[private]
default:
	@just --list --unsorted

test:
    cargo test
    cd tests-e2e && just

test-assets:
    cd test-assets && just
