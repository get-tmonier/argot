<?php

namespace Illuminate\Argotfix;

use yii\db\ActiveRecord;

class P02Yii extends ActiveRecord
{
    public static function tableName(): string
    {
        return "users";
    }
}
