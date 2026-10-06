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

HotDogStand is a desktop application. It reads and writes a workspace file,
and writes an export file. It opens no network connection. These faults are in
scope:

- A workspace file or an export file from another person runs code, or reads
  or writes a file that the person did not choose.
- A ticket or a comment in Markdown opens a link, runs a program, or loads a
  file without a click from the person.
- A click on a link in Markdown opens something that is not a web page or an
  e-mail address.
- The application opens a network connection.
- An export holds data that the person did not ask to export.
- A crash or a fault in a migration damages or loses the data in a workspace
  file.

## What is out of scope

- A person who can read the workspace file. That person can read every
  ticket. The file is not encrypted. Protect it with the permissions of your
  account, or with the disk encryption of your system.
- An export file that you send to another person. It is a copy of your data
  in clear text, on purpose.
- A fault in Slint, SQLite, or a dependency, when HotDogStand uses it in a
  safe way. Report it to that project.
