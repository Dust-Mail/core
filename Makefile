VERSION := 0.4.3

.PHONY: release

release:
	sed -i 's/^version = .*/version = "$(VERSION)"/' Cargo.toml
	cargo fetch
	git commit -am "bump(version): $(VERSION)"
	git tag -a v$(VERSION) -m "Version $(VERSION)"
	git push && git push --tags