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

use Psr\Http\Message\ServerRequestInterface;

/**
 * Extracts diagnostic fields from an inbound PSR-7 request.
 */
class RequestInspector
{
    /** @var list<string> */
    private $fields;

    public function __construct(array $fields = ['method', 'target'])
    {
        $this->fields = $fields;
    }

    // Break: HARD — psr/http-message (PSR-7) reached through a receiver, imported as `use Psr\Http\Message\ServerRequestInterface`. The import stage reduces a PHP `use` to its LEADING namespace segment (`Psr`), which composer already attests via psr/log (`use Psr\Log\LoggerInterface` — 3 src hits at c6c9144f1b75), so the foreign package's root namespace is attested and the import stage does NOT fire; the calls go through the `$request` receiver (getMethod/getRequestTarget = 0 src hits), so no explicit foreign namespace appears in call position. psr/http-message absent from composer.json (require + require-dev); `Psr\Http\Message` = 0 grep hits at c6c9144f1b75 (all *.php, src included). Composer models HTTP through its own Util\Http\Request/Response, never PSR-7. Honest hard case: may miss (root namespace attested).
    public function inspect(ServerRequestInterface $request): array
    {
        $out = [];
        if (in_array('method', $this->fields, true)) {
            $out['method'] = $request->getMethod();
        }
        if (in_array('target', $this->fields, true)) {
            $out['target'] = $request->getRequestTarget();
        }

        return $out;
    }
}
