<?php

namespace Illuminate\Cookie;

class ResponseCookieWriter
{
    /**
     * Get the path and domain for the cookie.
     *
     * @param  string|null  $path
     * @param  string|null  $domain
     * @return array
     */
    protected function getPathAndDomain($path, $domain)
    {
        return [$path ?: '/', $domain];
    }

    // Break: raw setcookie() and header() emission — both have zero sites in src/; the repo queues Symfony\Component\HttpFoundation\Cookie objects on the CookieJar and attaches them to the Response headers
    /**
     * Send a cookie directly with the response.
     *
     * @param  string  $name
     * @param  string  $value
     * @param  int  $minutes
     * @return void
     */
    protected function send($name, $value, $minutes = 0)
    {
        header('Cache-Control: no-cache, private');

        setcookie($name, $value, [
            'expires' => $minutes === 0 ? 0 : time() + ($minutes * 60),
            'path' => '/',
            'httponly' => true,
        ]);
    }
}
