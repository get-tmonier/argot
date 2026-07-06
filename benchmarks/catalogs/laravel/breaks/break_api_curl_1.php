<?php

namespace Illuminate\Http\Client;

class WebhookDispatcher
{
    /**
     * Build the payload for a webhook delivery.
     *
     * @param  array  $data
     * @return string
     */
    protected function payload(array $data)
    {
        return json_encode($data);
    }

    // Break: raw curl_init()/curl_setopt()/curl_exec() — curl_* has zero sites in src/; the repo's HTTP client issues requests through Guzzle-backed PendingRequest (Factory::pendingRequest, GuzzleHttp referenced 39 times in PendingRequest.php)
    /**
     * Deliver the given payload to a webhook URL.
     *
     * @param  string  $url
     * @param  string  $payload
     * @return string|false
     */
    protected function deliver($url, $payload)
    {
        $handle = curl_init($url);

        curl_setopt($handle, CURLOPT_POST, true);
        curl_setopt($handle, CURLOPT_POSTFIELDS, $payload);
        curl_setopt($handle, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($handle, CURLOPT_HTTPHEADER, ['Content-Type: application/json']);

        $response = curl_exec($handle);

        curl_close($handle);

        return $response;
    }
}
