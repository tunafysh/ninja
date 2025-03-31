package com.tunafysh

import java.awt.Font
import java.awt.GridBagConstraints
import java.awt.GridBagLayout
import java.awt.Insets
import javax.swing.*

class SplashScreen(private val frame: JFrame, private val label: JLabel, private val progressBar: JProgressBar) {
    fun update(title: String, progress: Int?) {
        SwingUtilities.invokeLater {
            label.text = title // Update the label text
            if (progress == null) {
                progressBar.isIndeterminate = true // Switch to indeterminate mode
                progressBar.isStringPainted = false
            } else {
                progressBar.isIndeterminate = false
                progressBar.isStringPainted = true
                progressBar.value = progress // Update the progress value
            }
        }
    }

    fun close() {
        SwingUtilities.invokeLater {
            frame.dispose() // Close the splash screen
        }
    }
}

fun createSplash(title: String, progress: Int?): SplashScreen {
    val frame = JFrame("")
    frame.defaultCloseOperation = JFrame.EXIT_ON_CLOSE
    frame.setSize(400, 200)
    frame.setLocationRelativeTo(null) // Center the frame on the screen

    // Use GridBagLayout for centering
    val panel = JPanel()
    panel.layout = GridBagLayout()
    val constraints = GridBagConstraints()
    constraints.gridx = 0
    constraints.gridy = GridBagConstraints.RELATIVE // Components stack vertically
    constraints.anchor = GridBagConstraints.CENTER // Center alignment
    constraints.insets = Insets(10, 0, 10, 0) // Add spacing around components

    // Create the label
    val label = JLabel(title)
    label.font = Font("Arial", Font.PLAIN, 14)
    panel.add(label, constraints) // Add the label with constraints

    // Create the progress bar
    val progressBar = JProgressBar()
    progressBar.minimum = 0
    progressBar.maximum = 100
    if (progress == null) progressBar.isIndeterminate = true else progressBar.value = progress
    progressBar.isStringPainted = false
    panel.add(progressBar, constraints) // Add the progress bar with constraints

    // Add the panel to the frame
    frame.add(panel)

    // Make the frame visible
    frame.isVisible = true

    return SplashScreen(frame, label, progressBar)
}

fun bootstrap() {
    var title = "Detecting Apache installation..."
    var progress: Int? = null

    val splashScreen = createSplash(title, progress)
    Thread.sleep(3000)

    title = "Loading environment..."
    progress = 50
    splashScreen.update(title, progress)
    Thread.sleep(3000)

    title = "Finalizing setup..."
    progress = 100
    splashScreen.update(title, progress)
    Thread.sleep(2000)

    splashScreen.close()
}