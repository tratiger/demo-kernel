import pexpect
import sys
import time

def main():
    print("Starting QEMU test...")
    child = pexpect.spawn('cargo run -Zjson-target-spec -Zbuild-std=core,alloc --target i686-os.json', encoding='utf-8', timeout=10)

    # Wait for the OS prompt
    try:
        child.expect('OMU-OS>', timeout=10)
        print("Prompt received.")

        # Test ls command
        child.sendline('ls')
        child.expect('test.txt', timeout=2)
        print("ls command verified.")

        # Wait for next prompt
        child.expect('OMU-OS>', timeout=2)

        # Test cat command
        child.sendline('cat test.txt')
        child.expect('Hello from test.txt inside initrd!', timeout=2)
        print("cat command verified.")

        print("All tests passed successfully!")
        child.terminate(force=True)
        sys.exit(0)
    except Exception as e:
        print(f"Test failed: {e}")
        print("QEMU output so far:")
        print(child.before)
        child.terminate(force=True)
        sys.exit(1)

if __name__ == '__main__':
    main()
