package com.tunafysh

import com.formdev.flatlaf.intellijthemes.FlatOneDarkIJTheme
import java.awt.BorderLayout
import javax.swing.JFrame
import javax.swing.JPanel
import javax.swing.SwingUtilities

fun test() {
    SwingUtilities.invokeLater({
    val frame = JFrame("Editor test")
    val cp = JPanel(BorderLayout())
    //val res = object {}.javaClass.getResource("/index.html")

//    cp.add(sp)

    frame.contentPane = cp
    frame.defaultCloseOperation = JFrame.EXIT_ON_CLOSE
    frame.pack()
    frame.setLocationRelativeTo(null)
    frame.isVisible = true
    })
}

fun main() {
    FlatOneDarkIJTheme.setup()
    bootstrap()
    test()
}