package hugolib
import "github.com/jinzhu/gorm"
func migrate(db *gorm.DB) { db.AutoMigrate(&struct{ ID int }{}) }
