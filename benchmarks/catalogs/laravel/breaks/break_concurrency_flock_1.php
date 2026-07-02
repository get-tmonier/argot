<?php

namespace Illuminate\Cache;

class ValueGuard
{
    /**
     * Format the key for a cache item.
     *
     * @param  string  $key
     * @return string
     */
    protected function itemKey($key)
    {
        return $key;
    }

    // Break: manual flock() file mutex with a usleep() busy-wait polling loop — flock() and usleep() have zero sites in src/; the repo acquires atomic locks via the Cache Lock contract and waits via Illuminate\Support\Sleep (7 files)
    /**
     * Execute the given callback while holding an exclusive lock.
     *
     * @param  string  $name
     * @param  callable  $callback
     * @return mixed
     */
    protected function withFileLock($name, callable $callback)
    {
        $handle = fopen(sys_get_temp_dir().'/'.$name.'.lock', 'c');

        while (! flock($handle, LOCK_EX | LOCK_NB)) {
            usleep(100000);
        }

        try {
            return $callback();
        } finally {
            flock($handle, LOCK_UN);
            fclose($handle);
        }
    }
}
