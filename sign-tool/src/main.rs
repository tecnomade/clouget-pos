use minisign::SecretKeyBox;
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: sign-tool <secret-key-file> <file-to-sign>");
        std::process::exit(1);
    }

    let sk_path = &args[1];
    let file_path = &args[2];

    eprintln!("Step 1: Reading key file...");
    let sk_box_str = fs::read_to_string(sk_path).expect("Cannot read secret key file");
    eprintln!("Step 2: Key file read ({} bytes)", sk_box_str.len());

    eprintln!("Step 3: Parsing key...");
    let sk_box = SecretKeyBox::from_string(&sk_box_str).expect("Cannot parse secret key");
    eprintln!("Step 4: Key parsed successfully");

    eprintln!("Step 5: Decrypting key (passwordless)...");
    let sk = sk_box.into_secret_key(Some("".to_string())).expect("Cannot decrypt secret key");
    eprintln!("Step 6: Key decrypted");

    eprintln!("Step 7: Reading file to sign...");
    let data = fs::read(file_path).expect("Cannot read file to sign");
    eprintln!("Step 8: File read ({} bytes)", data.len());
    let mut cursor = Cursor::new(&data);

    eprintln!("Step 9: Signing...");
    let sig_box = minisign::sign(None, &sk, &mut cursor, Some("Clouget Punto de Venta"), None)
        .expect("Cannot sign file");
    eprintln!("Step 10: Signed!");

    let sig_path = format!("{}.sig", file_path);
    let sig_string = sig_box.to_string();
    fs::write(&sig_path, &sig_string).expect("Cannot write signature");

    println!("Signature written to: {}", sig_path);
    println!("{}", sig_string);
}
