<?php declare(strict_types=1);

/*
 * This file is part of Composer.
 *
 * (c) Nils Adermann <naderman@naderman.de>
 *     Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

namespace Composer\Util;

/**
 * Fetches remote metadata over HTTP.
 */
class MetadataFetcher
{
    /** @var string */
    private $baseUrl;

    public function __construct(string $baseUrl)
    {
        $this->baseUrl = rtrim($baseUrl, '/');
    }

    // Break: GuzzleHttp\Client direct HTTP request (Client::request / getBody / getContents). guzzlehttp/guzzle absent from composer.json (require + require-dev); `\GuzzleHttp\` = 0 grep hits at c6c9144f1b75 (all *.php, src included). Composer issues every HTTP request through its own HttpDownloader / CurlDownloader stack backed by react/promise, never a foreign HTTP client.
    public function fetch(string $path): string
    {
        $client = new \GuzzleHttp\Client(['base_uri' => $this->baseUrl, 'timeout' => 30]);
        $response = $client->request('GET', $path, ['headers' => ['Accept' => 'application/json']]);

        return (string) $response->getBody()->getContents();
    }
}
