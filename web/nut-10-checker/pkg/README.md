# NUT-10 Spending Conditions Checker

Interactive web-based testing tool for Cashu spending conditions (NUT-10).

## Features

- **Multi-mint support**: Test against multiple Cashu mints
- **P2PK 2-of-2 Testing**: Verify 2-of-2 multisig enforcement
- **SigAll Testing**: Verify transaction integrity with SigAll flag
- **HTLC Testing**: Test hashed timelock contracts
- **Persistent results**: All test results stored in IndexedDB
- **Export/Import**: Export test results for sharing

## Building

Install wasm-pack:
```bash
cargo install wasm-pack
```

Build the WASM module:
```bash
./build.sh
```

## Running Locally

Start a local server:
```bash
python3 -m http.server 8000
```

Open in browser:
```
http://localhost:8000
```

## Test Cases

### P2PK 2-of-2 Multisig
1. ✅ Create 2-of-2 locked token
2. ❌ Try redeem with only Alice's signature (should fail)
3. ❌ Try redeem with only Bob's signature (should fail)
4. ✅ Redeem with both signatures (should succeed)

### SigAll Transaction Integrity
1. ✅ Create SigAll transaction
2. ❌ Try modify outputs after signing (should fail)
3. ❌ Try use wrong signature (should fail)
4. ✅ Complete with valid signatures (should succeed)

### HTLC
1. ✅ Create HTLC-locked token
2. ❌ Try redeem without preimage (should fail)
3. ❌ Try redeem with wrong preimage (should fail)
4. ✅ Redeem with correct preimage (should succeed)

## Deployment

The app can be deployed to GitHub Pages:

```bash
# Build for production
wasm-pack build --target web --release

# Commit and push to gh-pages branch
git checkout -b gh-pages
git add .
git commit -m "Deploy NUT-10 checker"
git push origin gh-pages
```

## License

Same as CDK parent project
