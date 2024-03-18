#!/usr/bin/python3
import sys, os, subprocess, time

def run(cmd, expect_failure=False):
    print('>>',cmd)
    proc = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    status = proc.wait() 
    out = proc.stdout.read().decode('utf-8')
    err = proc.stderr.read().decode('utf-8')
    result = (out+err).replace('\x00','\n') # for objcopy...
    sys.stdout.write(result)
    if status and not expect_failure:
        raise Exception(cmd,'FAILED')
    elif status == 0 and expect_failure:
        raise Exception(cmd,'was expected to fail but did not')
    return result

def run_and_time(cmd):
    start = time.time()
    run(cmd)
    return time.time() - start

def main():
    run('cargo test')
    run('cargo build --release')
    run('mkdir -p test/out')
    path = os.path.realpath(os.getcwd())+'/'
    path_size = (len(path) + 8) & ~7
    path = '/'*(path_size - len(path)) + path
    magic = 'OURMAGIC'*(path_size//8)
    prefix_flags = f'-fdebug-prefix-map=={magic} -ffile-prefix-map=={magic}'

    def sed_and_refix(what):
        path_for_sed = path.replace('/','\\/')
        run(f'cp test/out/{what} test/out/{what}.ref')
        sed_time = run_and_time(f'sed -i "s/{magic}/{path_for_sed}/g" test/out/{what}.ref')
        refix_time = run_and_time(f'cargo run --release test/out/{what} {magic} {path}')
        run(f'diff --brief test/out/{what} test/out/{what}.ref')
        return sed_time, refix_time

    # test that files are replaced in both debug info and assert/__FILE__,
    # that the output is the same as that of a simple sed run, and that
    # on the large file, refix is much faster than sed
    def test(prog):
        run(f'gcc -o test/out/{prog} test/test.c test/{prog}.c -g {prefix_flags}')
        out = run(f'./test/out/{prog}', expect_failure=True)
        assert magic in out and path not in out
        out = run(f'gdb -q test/out/{prog} -ex "list main" -ex quit')
        assert 'our source code' not in out

        sed_time, refix_time = sed_and_refix(prog)

        out = run(f'./test/out/{prog}', expect_failure=True)
        assert path in out and magic not in out
        out = run(f'gdb -q test/out/{prog} -ex "list main" -ex quit')
        assert 'our source code' in out

        return sed_time, refix_time

    # a ~3GB file with ~10% data to replace
    run("dd if=/dev/zero bs=1M count=3072 | tr '\\0' 'A' > test/out/As")
    run("dd if=/dev/zero bs=1M count=307 | tr '\\0' 'B' >> test/out/As")
    t = run_and_time("cargo run test/out/As "+'B'*100+" "+'C'*100)
    print('a 3GB file processed fully, without skipping sections took',t,'seconds')
    run('rm test/out/As')

    # test --section
    run('gcc -c test/data.c -o test/out/data.o -g')
    def get_section(name):
        return run(f'objcopy --dump-section={name}=/dev/stdout test/out/data.o').strip()
    assert get_section('.section_to_replace') == 'ORIGDATA'
    assert get_section('.another_section_to_replace') == 'ORIGDATA'
    run('echo "NEW DATA" > test/out/data1')
    run('echo "ANOTHER!" > test/out/data2')
    run(f'cargo run test/out/data.o {magic} {path} --section .section_to_replace test/out/data1 --section .another_section_to_replace test/out/data2')
    assert get_section('.section_to_replace') == 'NEW DATA'
    assert get_section('.another_section_to_replace') == 'ANOTHER!'

    # test support for the ar format
    run('gcc -c test/small.c -o test/out/small.o')
    run('gcc -c test/test.c -o test/out/test.o -Dmain=whatever')
    run('ar crs test/out/libtest.a test/out/small.o test/out/test.o')
    sed_and_refix('libtest.a')

    # test on an small & a large executable incl gdb & assert
    test('small')
    sed_time, refix_time = test('large')
    print(f'sed took {sed_time} seconds')
    print(f'refix took {refix_time} seconds (speedup: {sed_time/refix_time}x)')
    assert sed_time/refix_time >= 10, 'refix should be at least 10x faster on the larger file (since it ignores most of it)'

if __name__ == '__main__':
    main()
