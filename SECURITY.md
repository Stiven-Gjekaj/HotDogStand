<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Security Policy

## Versions that get a fix

HotDogStand is early. A security fix goes to the newest commit on the default
branch. No older version is maintained.

## How to report a problem

Report a security problem in private. Do not open a public issue.

- Best: open a private advisory with the "Report a vulnerability" button on the
  Security tab of the repository.
- Or write to the maintainer at stivenagostingjekaj@gmail.com.

Say how to make the problem happen again, which commit you used, and what you
believe the effect is. You can expect a first answer within a few days. Your
report is named in the fix unless you ask to stay anonymous.

## What is in scope

HotDogStand is a web server that keeps accounts and the text that people
write. These faults are in scope:

- A person reads or changes a ticket, a comment, or an account without a
  session.
- A member does a thing that only an admin can do.
- Text in a ticket or a comment runs a script in the browser of another
  person.
- A form changes data without its CSRF token.
- A session continues after log out, or after an admin deactivates the user.
- A request reads or writes a file outside the data directory.
- A password or a session token goes into a log or into the database as clear
  text.

## What is out of scope

- A server that you run on plain HTTP on a public network. Put it behind
  HTTPS and set `HOTDOGSTAND_SECURE_COOKIES=true`.
- A person who can read the database file. That person can read every ticket.
  Protect the file with the permissions of the operating system.
- An admin who does what an admin can do.
- A fault in 7.css, Python, or a dependency, when HotDogStand uses it in a safe
  way. Report it to that project.
