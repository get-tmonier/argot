<?php

namespace Illuminate\Argotfix;

use Laminas\Mvc\Application;

class P05Laminas
{
    public function boot(array $config): Application
    {
        return Application::init($config);
    }
}
