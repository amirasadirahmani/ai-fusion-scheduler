.PHONY: validate build-userspace build-sched-ext simulator-smoke smoke analyze zip clean

validate:
	./scripts/validate_package.sh

build-userspace:
	./scripts/build_userspace.sh

build-sched-ext:
	./scripts/build_sched_ext.sh

simulator-smoke:
	cargo run -p afs-policy-simulator -- --config configs/smoke.toml --output results/simulator-smoke.csv

smoke:
	sudo ./scripts/run_smoke_test.sh

analyze:
	python3 analysis/analyze_results.py results --out results/run-summary.csv --aggregate-out results/aggregate-summary.csv

zip:
	./scripts/make_release_zip.sh

clean:
	rm -rf target results/* run/* analysis/__pycache__ scripts/__pycache__
	touch results/.gitkeep run/.gitkeep
