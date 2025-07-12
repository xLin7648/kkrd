plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.mozilla.rust-android-gradle.rust-android")
}

android {
    namespace = "com.example.wgpuapplication"
    compileSdk = 34
    ndkVersion = "28.0.12674087"

    defaultConfig {
        applicationId = "com.example.wgpuapplication"
        minSdk = 32
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        viewBinding = true
    }
    sourceSets {
        getByName("androidTest") {
            jniLibs.srcDir("$buildDir/rustJniLibs/android")
        }
        getByName("debug") {
            jniLibs.srcDir("$buildDir/rustJniLibs/android")
        }
    }
}

cargo {
    module  = "../lib"
    libname = "wgpu_android_lib"
    targets = listOf("arm64")
}

tasks.whenTaskAdded {
    if (name == "javaPreCompileDebug" || name == "javaPreCompileRelease") {
        dependsOn("cargoBuild")
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.games:games-activity:2.0.2")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.oboe:oboe:1.5.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}