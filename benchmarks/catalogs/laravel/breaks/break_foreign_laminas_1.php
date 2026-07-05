<?php

namespace Illuminate\Http;

use Laminas\Diactoros\Response;

class Psr7ResponseBuilder
{
    /**
     * Resolve the default headers for a JSON response.
     *
     * @return array
     */
    protected function jsonHeaders()
    {
        return ['Content-Type' => 'application/json'];
    }

    // Break: laminas/laminas-diactoros PSR-7 response — laminas/laminas-diactoros absent from composer.json (require + require-dev); Laminas\ has zero hits in src/ at the pinned SHA. The foreign class is named `Response`, colliding with the attested Illuminate\Http\Response the repo defines and returns, so the foreign PSR-7 object is masked behind a repo-owned type name; the repo builds responses through its own Response class over Symfony HttpFoundation.
    /**
     * Build a JSON response for the given payload.
     *
     * @param  array  $payload
     * @param  int  $status
     * @return \Psr\Http\Message\ResponseInterface
     */
    protected function buildJson(array $payload, $status = 200)
    {
        $response = new Response();

        $response->getBody()->write(json_encode($payload));

        return $response
            ->withStatus($status)
            ->withHeader('Content-Type', 'application/json');
    }
}
