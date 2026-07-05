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

namespace Composer\Plugin;

use Pimple\Container as ServiceContainer;

/**
 * Wires plugin service factories into a dependency container.
 */
class PluginServiceContainer
{
    /** @var array<string, callable> */
    private $factories = [];

    public function register(string $id, callable $factory): void
    {
        $this->factories[$id] = $factory;
    }

    // Break: pimple/pimple DI container reached through an aliased import (use Pimple\Container as ServiceContainer; new ServiceContainer()); the foreign dependency is not an FQN in call position. pimple/pimple absent from composer.json (require + require-dev); `Pimple` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `Pimple\Container` = 0 hits. Composer wires plugin capabilities through its own PluginManager / Capable interface, never a foreign service container.
    public function build(): ServiceContainer
    {
        $container = new ServiceContainer();
        foreach ($this->factories as $id => $factory) {
            $container[$id] = static function () use ($factory) {
                return $factory();
            };
        }

        return $container;
    }
}
