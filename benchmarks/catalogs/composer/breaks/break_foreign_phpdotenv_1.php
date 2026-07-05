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
 * Seeds configuration overrides from a project .env file.
 */
class EnvConfigLoader
{
    /** @var string */
    private $projectDir;

    public function __construct(string $projectDir)
    {
        $this->projectDir = $projectDir;
    }

    // Break: vlucas/phpdotenv environment loading via FQN (Dotenv::createImmutable(...)->load() + required()). vlucas/phpdotenv absent from composer.json (require + require-dev); `\Dotenv\` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Dotenv::createImmutable` = 0 hits. Composer reads environment overrides through Platform::getEnv / getenv in its own Config layer, never a foreign dotenv library.
    public function load(): array
    {
        $dotenv = \Dotenv\Dotenv::createImmutable($this->projectDir);
        $dotenv->load();
        $dotenv->required(['COMPOSER_HOME']);

        return $_ENV;
    }
}
