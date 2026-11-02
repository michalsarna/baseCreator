package com.creatorapps.basecreator;

import com.google.androidgamesdk.GameActivity;

public class MainActivity extends GameActivity {
    static {
        System.loadLibrary("basecreator");
    }
}
