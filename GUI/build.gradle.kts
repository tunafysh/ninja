plugins {
    kotlin("jvm") version "2.1.10"
}

group = "com.tunafysh"
version = "0.5"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))
    implementation("com.formdev:flatlaf:3.5.4")
    implementation("com.formdev:flatlaf-intellij-themes:3.5.4")
    // https://mvnrepository.com/artifact/org.openjfx/javafx-controls
    implementation("org.openjfx:javafx-controls:21")
}

tasks.test {
    useJUnitPlatform()
}
kotlin {
    jvmToolchain(21)
}