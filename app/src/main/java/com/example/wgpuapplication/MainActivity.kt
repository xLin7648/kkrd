package com.example.wgpuapplication

import android.app.NativeActivity // 1. 导入NativeActivity
import android.content.pm.ActivityInfo
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import androidx.annotation.RequiresApi
import androidx.core.view.ViewCompat

class MainActivity : NativeActivity() {
    companion object {
        init {
            Log.i("TEST", "TEST")
            System.loadLibrary("wgpu_android_lib") // 3. 保留库加载
        }
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            hideSystemUi()
            setupDisplayCutoutHandling()
        }
    }
    // 在 Activity 的 onCreate 中调用
    fun setupDisplayCutoutHandling() {
        // 1. 设置窗口延伸至刘海区域
        window.apply {
            // 关键设置：允许内容延伸到短边刘海区域
            attributes = attributes.apply {
                layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            }
        }
    }

    private fun hideSystemUi() {
        val decorView = window.decorView
        decorView.systemUiVisibility = (
                View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                        or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                        or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                        or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                        or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        or View.SYSTEM_UI_FLAG_FULLSCREEN
                )
    }
}
