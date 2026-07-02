<?php

namespace Illuminate\Config;

use Illuminate\Support\Arr;

class RepositoryReader
{
    /**
     * Determine if the given configuration value exists.
     *
     * @param  string  $key
     * @return bool
     */
    public function has($key)
    {
        return Arr::has($this->items ?? [], $key);
    }

    // Break: trigger_error(E_USER_ERROR) for a missing config key — the repo's 3 trigger_error sites are all E_USER_WARNING deprecation/pool notices; required-value failures throw InvalidArgumentException
    /**
     * Get the specified configuration value or fail.
     *
     * @param  string  $key
     * @return mixed
     */
    public function getOrFail($key)
    {
        $value = Arr::get($this->items ?? [], $key);

        if ($value === null) {
            trigger_error("Configuration value for key [{$key}] has no value.", E_USER_ERROR);
        }

        return $value;
    }
}
