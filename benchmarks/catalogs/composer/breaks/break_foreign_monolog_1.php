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

namespace Composer;

/**
 * Records cache subsystem diagnostics.
 */
class CacheAuditLog
{
    /** @var string */
    private $root;

    public function __construct(string $root)
    {
        $this->root = $root;
    }

    // Break: Monolog logger stack (Logger + StreamHandler + pushHandler) for cache diagnostics. monolog/monolog absent from composer.json (require + require-dev); `\Monolog\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `pushHandler` = 0 hits. Composer reports diagnostics exclusively through the injected IOInterface (io->writeError), never a foreign logging framework.
    public function record(string $message): void
    {
        $logger = new \Monolog\Logger('composer.cache');
        $logger->pushHandler(new \Monolog\Handler\StreamHandler($this->root . '/cache.log', \Monolog\Logger::WARNING));
        $logger->pushProcessor(new \Monolog\Processor\PsrLogMessageProcessor());

        $logger->warning($message, ['root' => $this->root]);
    }
}
