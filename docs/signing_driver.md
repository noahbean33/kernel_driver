# Module Signing for Secure Boot

If you are on a virtual machine or cannot access the BIOS, you can create a local trusted key and sign the module yourself. This is more involved but keeps your system secure while allowing you to load custom kernel modules.

## Step 1: Generate a Key Pair

Create an X.509 key pair for signing your modules:

```bash
openssl req -new -x509 -newkey rsa:2048 \
  -keyout MOK.priv \
  -outform DER \
  -out MOK.der \
  -nodes \
  -days 36500 \
  -subj "/CN=MyCustomKey/"
```

This generates:

- **MOK.priv**: Private key (keep secure)
- **MOK.der**: Public certificate (DER format)

## Step 2: Sign Your Module

Locate the `sign-file` tool in your kernel build directory and sign the module:

```bash
sudo /usr/src/linux-headers-$(uname -r)/scripts/sign-file \
  sha256 \
  ./MOK.priv \
  ./MOK.der \
  hello.ko
```

**Note**: The path to `sign-file` is standard for Ubuntu/Debian. On other distributions, check your kernel headers package.

## Step 3: Import the Key

Enroll your public certificate with MOK (Machine Owner Key):

```bash
sudo mokutil --import MOK.der
```

You will be prompted to create a password. Choose something simple (e.g., `1234`) - you'll only use it once during the next step.

## Step 4: Enroll the Key (Requires Reboot)

1. **Reboot your machine**:

   ```bash
   sudo reboot
   ```

2. **MOK Management Menu**: Upon restart, you will see a blue screen called the **MOK Management** menu.

3. **Enroll the key**:
   - Select **Enroll MOK**
   - Select **Continue**
   - Select **Yes**
   - Enter the password you created in Step 3

4. **Reboot again**: The system will reboot once more.

Now, your kernel will trust any module signed with your `MOK.priv` key.

## Verifying Your Work

After successfully loading the module:

```bash
sudo insmod ./hello.ko
```

Verify it loaded by checking the kernel logs:

```bash
dmesg | tail -n 1
```

**Expected output:**

```text
[ 1234.567] Hello World!
```

## Troubleshooting

### Module Still Won't Load

- Check if Secure Boot is enabled: `mokutil --sb-state`
- Verify the module is signed: `modinfo hello.ko | grep sig`
- Check `dmesg` for signature verification errors

### Lost MOK Password

The password is only needed during enrollment. If you've already enrolled the key, you don't need the password again.

## Security Note

Keep your `MOK.priv` file secure. Anyone with access to this private key can sign kernel modules that your system will trust.
