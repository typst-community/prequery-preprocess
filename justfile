[private]
default:
	@just --list --unsorted

test:
    cd tests && just

test-assets:
    cd test-assets && just
