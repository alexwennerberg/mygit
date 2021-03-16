// TODO write standalone server -- routes etc should live in a separate module
// TODO one-way mailbox sync
use email_parser::email::Email;

fn main() {
    let email = Email::parse(
        b"\
    From: Mubelotix <mubelotix@mubelotix.dev>\r\n\
    Subject:Example Email\r\n\
    To: Someone <example@example.com>\r\n\
    Message-id: <6546518945@mubelotix.dev>\r\n\
    Date: 5 May 2003 18:58:34 +0000\r\n\
    \r\n\
    Hey!\r\n",
    )
    .unwrap();

    assert_eq!(email.subject.unwrap(), "Example Email");
    assert_eq!(email.sender.name.unwrap(), vec!["Mubelotix"]);
    assert_eq!(email.sender.address.local_part, "mubelotix");
    assert_eq!(email.sender.address.domain, "mubelotix.dev");
    println!("Hello, world!");
}
