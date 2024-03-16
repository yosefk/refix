#!/usr/bin/python3
import sys, os, subprocess, time

def run(cmd, expect_failure=False):
    print('>>',cmd)
    proc = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    status = proc.wait() 
    if status and not expect_failure:
        raise Exception(cmd,'FAILED')
    elif status == 0 and expect_failure:
        raise Exception(cmd,'was expected to fail but did not')
    out = proc.stdout.read().decode('utf-8')
    err = proc.stderr.read().decode('utf-8')
    sys.stdout.write(out)
    sys.stdout.write(err)
    return out + err

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
        sed_time = run_and_time(f'sed "s/{magic}/{path_for_sed}/g" test/out/{what} > test/out/{what}.ref')
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
