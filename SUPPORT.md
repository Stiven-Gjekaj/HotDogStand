<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Getting help

## Read first

- [README.md](README.md) says what HotDogStand is and how to start it.
- [docs/architecture.md](docs/architecture.md) explains the parts, the data
  model, and the settings.
- [TODO.md](TODO.md) says what is not built yet. Look here before you report
  that something is missing.

## When the server refuses to start

HotDogStand writes one line for a problem that you can correct, and it says
what to change. Two of these are common:

- **A setting that nothing reads.** A variable that starts with
  `HOTDOGSTAND_` is spelled wrong. The message names the variable that you
  probably meant.
- **A database that is older than the code.** Run `hotdogstand migrate`.

## When you cannot log in

The page gives one message for a wrong name and for a wrong password. This is
on purpose. Ask an admin to check that your account is active. An admin who
lost their password runs `hotdogstand create-user --admin` on the server to
make a new admin account.

## Ask a question or report a fault

- Look through the
  [issues](https://github.com/Stiven-Gjekaj/HotDogStand/issues) first.
- Open a bug report for a fault, or a feature request for something new.

Say which commit you used, which browser you used, what you did, and what
happened. A screenshot helps for a fault in the look.

Do not use the issue tracker for a security problem. See
[SECURITY.md](SECURITY.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
