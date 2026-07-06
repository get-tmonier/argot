<?php

namespace Illuminate\Http;

use Illuminate\Support\Arr;

class InputResolver
{
    /**
     * Retrieve a subset of the given input array.
     *
     * @param  array  $input
     * @param  array  $keys
     * @return array
     */
    protected function only(array $input, array $keys)
    {
        return Arr::only($input, $keys);
    }

    // Break: raw $_GET/$_POST/$_SERVER superglobals — zero sites in src/; the repo reads input through the Request abstraction (input()/query()/header() on Symfony HttpFoundation)
    /**
     * Retrieve an input item from the current request.
     *
     * @param  string  $key
     * @param  mixed  $default
     * @return mixed
     */
    protected function inputFromGlobals($key, $default = null)
    {
        if (isset($_POST[$key])) {
            return $_POST[$key];
        }

        if (isset($_GET[$key])) {
            return $_GET[$key];
        }

        if ($_SERVER['REQUEST_METHOD'] === 'HEAD') {
            return $default;
        }

        return $default;
    }
}
